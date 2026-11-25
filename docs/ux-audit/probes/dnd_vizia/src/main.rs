//! Probe: OS file drag-and-drop on a vizia window (ChipDAW's vizia fork), and the smallest
//! fix for "where is the cursor while a file hovers".
//!
//! Modes:
//! - `dnd_vizia` (stock): logs what vizia delivers: `WindowEvent::MouseMove/MouseEnter/
//!   MouseLeave/Drop` at the root, and per-view `.on_drop` on two panes.
//! - `dnd_vizia --own-droptarget`: revokes winit's IDropTarget on the HWND and registers our
//!   own (≈120 lines below). DragEnter/DragOver/DragLeave/Drop carry the cursor in screen
//!   coordinates; they are converted to client px and emitted into vizia through a
//!   ContextProxy, so the UI can track the hover (a label shows it live).
//!
//! Window title "DndViziaProbe"; log: `dnd_vizia.log` in the cwd (wall-clock times, so it
//! lines up with tools/oledrag.ps1's output).

#![windows_subsystem = "windows"]

use std::ffi::c_void;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use vizia::prelude::*;
use windows_sys::core::{GUID, HRESULT};
use windows_sys::Win32::Foundation::{E_NOINTERFACE, HWND, POINT, POINTL, S_OK};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::Com::{DVASPECT_CONTENT, FORMATETC, STGMEDIUM, TYMED_HGLOBAL};
use windows_sys::Win32::System::Ole::{
    ReleaseStgMedium, RegisterDragDrop, RevokeDragDrop, CF_HDROP, DROPEFFECT_COPY, DROPEFFECT_NONE,
};
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Shell::{DragQueryFileW, HDROP};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

// ---------------------------------------------------------------- logging

static LOG: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

fn log(msg: impl AsRef<str>) {
    let t = unsafe {
        let mut st = std::mem::zeroed();
        GetLocalTime(&mut st);
        st
    };
    let line = format!(
        "{:02}:{:02}:{:02}.{:03} {}",
        t.wHour, t.wMinute, t.wSecond, t.wMilliseconds,
        msg.as_ref()
    );
    if let Some(f) = LOG.get() {
        let _ = writeln!(f.lock().unwrap(), "{line}");
    }
}

// ---------------------------------------------------------------- our IDropTarget

/// Events our drop target sends into vizia.
#[derive(Debug, Clone)]
enum OsDrag {
    Enter { files: Vec<PathBuf>, x: i32, y: i32 },
    Over { x: i32, y: i32 },
    Leave,
    Drop { files: Vec<PathBuf>, x: i32, y: i32 },
}

#[repr(C)]
struct DropTargetVtbl {
    query_interface: unsafe extern "system" fn(*mut DropTarget, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut DropTarget) -> u32,
    release: unsafe extern "system" fn(*mut DropTarget) -> u32,
    // The real IDropTarget ABI passes POINTL by value.
    drag_enter: unsafe extern "system" fn(*mut DropTarget, *mut c_void, u32, POINTL, *mut u32) -> HRESULT,
    drag_over: unsafe extern "system" fn(*mut DropTarget, u32, POINTL, *mut u32) -> HRESULT,
    drag_leave: unsafe extern "system" fn(*mut DropTarget) -> HRESULT,
    drop: unsafe extern "system" fn(*mut DropTarget, *mut c_void, u32, POINTL, *mut u32) -> HRESULT,
}

#[repr(C)]
struct DropTarget {
    vtbl: *const DropTargetVtbl,
    refs: AtomicU32,
    hwnd: isize,
    effect: u32,
    proxy: Mutex<ContextProxy>,
}

const IID_IUNKNOWN: GUID = GUID::from_u128(0x00000000_0000_0000_c000_000000000046);
const IID_IDROPTARGET: GUID = GUID::from_u128(0x00000122_0000_0000_c000_000000000046);

static VTBL: DropTargetVtbl = DropTargetVtbl {
    query_interface: dt_query_interface,
    add_ref: dt_add_ref,
    release: dt_release,
    drag_enter: dt_drag_enter,
    drag_over: dt_drag_over,
    drag_leave: dt_drag_leave,
    drop: dt_drop,
};

fn guid_eq(a: &GUID, b: &GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

unsafe extern "system" fn dt_query_interface(this: *mut DropTarget, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    let iid = unsafe { &*iid };
    if guid_eq(iid, &IID_IUNKNOWN) || guid_eq(iid, &IID_IDROPTARGET) {
        unsafe {
            dt_add_ref(this);
            *out = this as *mut c_void;
        }
        S_OK
    } else {
        unsafe { *out = std::ptr::null_mut() };
        E_NOINTERFACE
    }
}

unsafe extern "system" fn dt_add_ref(this: *mut DropTarget) -> u32 {
    unsafe { (*this).refs.fetch_add(1, Ordering::AcqRel) + 1 }
}

unsafe extern "system" fn dt_release(this: *mut DropTarget) -> u32 {
    let n = unsafe { (*this).refs.fetch_sub(1, Ordering::AcqRel) - 1 };
    if n == 0 {
        drop(unsafe { Box::from_raw(this) });
    }
    n
}

/// CF_HDROP file list of a data object, or None if it carries no files.
unsafe fn files_of(data: *mut c_void) -> Option<Vec<PathBuf>> {
    #[repr(C)]
    struct DataObjectVtbl {
        _qi: usize,
        _add_ref: usize,
        _release: usize,
        get_data: unsafe extern "system" fn(*mut c_void, *const FORMATETC, *mut STGMEDIUM) -> HRESULT,
    }
    let vtbl = unsafe { &**(data as *mut *const DataObjectVtbl) };
    let fmt = FORMATETC {
        cfFormat: CF_HDROP,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT as u32,
        lindex: -1,
        tymed: TYMED_HGLOBAL as u32,
    };
    let mut medium: STGMEDIUM = unsafe { std::mem::zeroed() };
    if unsafe { (vtbl.get_data)(data, &fmt, &mut medium) } != S_OK {
        return None;
    }
    let hdrop = unsafe { medium.u.hGlobal } as HDROP;
    let n = unsafe { DragQueryFileW(hdrop, u32::MAX, std::ptr::null_mut(), 0) };
    let mut out = Vec::new();
    for i in 0..n {
        let len = unsafe { DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0) } as usize + 1;
        let mut buf = vec![0u16; len];
        unsafe { DragQueryFileW(hdrop, i, buf.as_mut_ptr(), len as u32) };
        buf.pop();
        out.push(PathBuf::from(String::from_utf16_lossy(&buf)));
    }
    unsafe { ReleaseStgMedium(&mut medium) };
    Some(out)
}

fn to_client(hwnd: isize, pt: POINTL) -> (i32, i32) {
    let mut p = POINT { x: pt.x, y: pt.y };
    unsafe { ScreenToClient(hwnd as HWND, &mut p) };
    (p.x, p.y)
}

fn send(this: *mut DropTarget, e: OsDrag) {
    let _ = unsafe { &*this }.proxy.lock().unwrap().emit(e);
}

unsafe extern "system" fn dt_drag_enter(this: *mut DropTarget, data: *mut c_void, _keys: u32, pt: POINTL, effect: *mut u32) -> HRESULT {
    let files = unsafe { files_of(data) };
    let (x, y) = to_client(unsafe { (*this).hwnd }, pt);
    log(format!("IDropTarget::DragEnter screen ({}, {}) client ({x}, {y}) files={files:?}", pt.x, pt.y));
    let eff = if files.is_some() { DROPEFFECT_COPY } else { DROPEFFECT_NONE };
    unsafe {
        (*this).effect = eff;
        *effect = eff;
    }
    if let Some(files) = files {
        send(this, OsDrag::Enter { files, x, y });
    }
    S_OK
}

unsafe extern "system" fn dt_drag_over(this: *mut DropTarget, _keys: u32, pt: POINTL, effect: *mut u32) -> HRESULT {
    let (x, y) = to_client(unsafe { (*this).hwnd }, pt);
    log(format!("IDropTarget::DragOver  screen ({}, {}) client ({x}, {y})", pt.x, pt.y));
    unsafe { *effect = (*this).effect };
    send(this, OsDrag::Over { x, y });
    S_OK
}

unsafe extern "system" fn dt_drag_leave(this: *mut DropTarget) -> HRESULT {
    log("IDropTarget::DragLeave");
    send(this, OsDrag::Leave);
    S_OK
}

unsafe extern "system" fn dt_drop(this: *mut DropTarget, data: *mut c_void, _keys: u32, pt: POINTL, effect: *mut u32) -> HRESULT {
    let files = unsafe { files_of(data) }.unwrap_or_default();
    let (x, y) = to_client(unsafe { (*this).hwnd }, pt);
    log(format!("IDropTarget::Drop      screen ({}, {}) client ({x}, {y}) files={files:?}", pt.x, pt.y));
    unsafe { *effect = DROPEFFECT_COPY };
    send(this, OsDrag::Drop { files, x, y });
    S_OK
}

// ---------------------------------------------------------------- vizia side

static HWND_MAIN: AtomicIsize = AtomicIsize::new(0);

struct Probe {
    own_target: bool,
    installed: bool,
    status: Signal<String>,
}

#[derive(Debug)]
struct Init;

impl Model for Probe {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|_: &Init, _| {
            if self.installed {
                return;
            }
            let Some(win) = cx.window() else { return };
            let RawWindowHandle::Win32(h) = win.window_handle().unwrap().as_raw() else { unreachable!() };
            let hwnd = h.hwnd.get();
            HWND_MAIN.store(hwnd, Ordering::Relaxed);
            log(format!("Init: HWND=0x{hwnd:X}, own drop target: {}", self.own_target));
            if self.own_target {
                // winit registered its FileDropHandler at window creation (winit 0.30.13
                // platform_impl/windows/window.rs:1194). Replace it with ours; winit still
                // calls RevokeDragDrop on WM_DESTROY (event_loop.rs:1262), which releases ours.
                let revoke = unsafe { RevokeDragDrop(hwnd as HWND) };
                let target = Box::into_raw(Box::new(DropTarget {
                    vtbl: &VTBL,
                    refs: AtomicU32::new(1),
                    hwnd,
                    effect: DROPEFFECT_NONE,
                    proxy: Mutex::new(cx.get_proxy()),
                }));
                let reg = unsafe { RegisterDragDrop(hwnd as HWND, target as *mut c_void) };
                // RegisterDragDrop took its own reference; drop ours.
                unsafe { dt_release(target) };
                log(format!("Init: RevokeDragDrop=0x{revoke:X} RegisterDragDrop=0x{reg:X}"));
            }
            self.installed = true;
        });
        event.map(|e: &OsDrag, _| {
            log(format!("vizia Model: OsDrag::{e:?}"));
            match e {
                OsDrag::Enter { x, y, .. } | OsDrag::Over { x, y } => {
                    self.status.set(format!("file hovering at client ({x}, {y})"))
                }
                OsDrag::Leave => self.status.set("hover cancelled".into()),
                OsDrag::Drop { x, y, .. } => self.status.set(format!("dropped at client ({x}, {y})")),
            }
        });
        event.map(|we: &WindowEvent, _| match we {
            WindowEvent::MouseMove(x, y) => log(format!("vizia WindowEvent::MouseMove({x:.0}, {y:.0})")),
            WindowEvent::MouseEnter => log("vizia WindowEvent::MouseEnter"),
            WindowEvent::MouseLeave => log("vizia WindowEvent::MouseLeave"),
            WindowEvent::Drop(d) => {
                let m = cx.mouse();
                log(format!(
                    "vizia WindowEvent::Drop({d:?}); cx.mouse() cursor at that moment ({:.0}, {:.0})",
                    m.cursor_x, m.cursor_y
                ));
            }
            _ => {}
        });
    }
}

fn pane(cx: &mut Context, name: &'static str) {
    VStack::new(cx, |cx| {
        Label::new(cx, name);
    })
    .width(Stretch(1.0))
    .height(Stretch(1.0))
    .border_width(Pixels(2.0))
    .border_color(Color::rgb(0, 245, 160))
    .on_drop(move |cx, data| {
        let m = cx.mouse();
        log(format!(
            "vizia .on_drop on {name}: {data:?} (cursor ({:.0}, {:.0}))",
            m.cursor_x, m.cursor_y
        ));
    });
}

fn main() -> Result<(), ApplicationError> {
    let _ = LOG.set(Mutex::new(std::fs::File::create("dnd_vizia.log").unwrap()));
    let own_target = std::env::args().any(|a| a == "--own-droptarget");
    log(format!("start; own drop target: {own_target}"));
    Application::new(move |cx| {
        let status = Signal::new(String::from("no OS drag yet"));
        Probe { own_target, installed: false, status }.build(cx);
        VStack::new(cx, |cx| {
            Label::new(cx, status).height(Pixels(28.0));
            HStack::new(cx, |cx| {
                pane(cx, "LEFT pane (.on_drop)");
                pane(cx, "RIGHT pane (.on_drop)");
            });
        });
        cx.emit(Init);
    })
    .title("DndViziaProbe")
    .inner_size((700, 450))
    // Right half of a 2560 px screen, clear of a running ChipDAW.
    .position((1600, 200))
    .run()
}
