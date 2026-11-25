//! Probe: a native Win32 menu bar (muda 0.22) on a vizia window (ChipDAW's vizia fork).
//!
//! Answers: how to get the HWND from vizia, how muda's MenuEvent reaches vizia's event
//! system, whether accelerators double-fire with vizia's key handling, whether the menu bar
//! changes the client area / layout, and whether the menu's modal loop pauses rendering.
//!
//! Run: `menu_vizia [--accel none|hook|hook-swallow]` (default none). Log: `menu_vizia.log`
//! in the current directory (and stdout).
//!
//! - `none`: muda's accelerator table is never consulted (vizia builds winit's EventLoop
//!   itself, so `EventLoopBuilderExtWindows::with_msg_hook` is unavailable).
//! - `hook`: a thread-local WH_GETMESSAGE hook calls `TranslateAcceleratorW` but lets the
//!   key message through (shows the double-fire).
//! - `hook-swallow`: same, but a translated key message is turned into WM_NULL.

use std::io::Write;
use std::sync::atomic::{AtomicIsize, AtomicU32, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use muda::accelerator::Accelerator;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use vizia::prelude::*;
use vizia::vg;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

// ---------------------------------------------------------------- logging

static START: OnceLock<Instant> = OnceLock::new();
static LOG: OnceLock<Mutex<std::fs::File>> = OnceLock::new();

fn log(msg: impl AsRef<str>) {
    let t = START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0;
    let tid = unsafe { GetCurrentThreadId() };
    let line = format!("{t:10.1} ms [tid {tid:>5}] {}", msg.as_ref());
    println!("{line}");
    if let Some(f) = LOG.get() {
        let _ = writeln!(f.lock().unwrap(), "{line}");
    }
}

// ---------------------------------------------------------------- shared state

static HWND_MAIN: AtomicIsize = AtomicIsize::new(0);
static HACCEL: AtomicIsize = AtomicIsize::new(0);
/// 0 = none, 1 = hook (no swallow), 2 = hook + swallow.
static ACCEL_MODE: AtomicU8 = AtomicU8::new(0);
static FRAMES: AtomicU32 = AtomicU32::new(0);
static TICKS: AtomicU32 = AtomicU32::new(0);
/// Last draw / timer tick, in µs since START.
static LAST_DRAW_US: AtomicU64 = AtomicU64::new(0);
static LAST_TICK_US: AtomicU64 = AtomicU64::new(0);
/// A gap above this between draws / timer ticks is logged as a stall.
const STALL: Duration = Duration::from_millis(100);
/// Redraw pump interval (about one 60 Hz frame).
const PUMP: Duration = Duration::from_millis(16);

fn now_us() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_micros() as u64
}

fn client_rect(hwnd: isize) -> (i32, i32) {
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { GetClientRect(hwnd as HWND, &mut r) };
    (r.right - r.left, r.bottom - r.top)
}

// ---------------------------------------------------------------- Win32 hooks

/// Thread WH_GETMESSAGE hook: the only way to run `TranslateAcceleratorW` inside winit's
/// message loop when the EventLoop is built by vizia (no `with_msg_hook`).
unsafe extern "system" fn getmsg_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam == PM_REMOVE as usize {
        let msg = unsafe { &mut *(lparam as *mut MSG) };
        if msg.message == WM_KEYDOWN || msg.message == WM_SYSKEYDOWN {
            let haccel = HACCEL.load(Ordering::Relaxed);
            let hwnd = HWND_MAIN.load(Ordering::Relaxed);
            if haccel != 0 && hwnd != 0 {
                let r = unsafe { TranslateAcceleratorW(hwnd as HWND, haccel as HACCEL, msg) };
                if r != 0 {
                    let swallow = ACCEL_MODE.load(Ordering::Relaxed) == 2;
                    log(format!(
                        "hook: TranslateAcceleratorW translated vk=0x{:02X} (WM_COMMAND sent); swallow={swallow}",
                        msg.wParam
                    ));
                    if swallow {
                        msg.message = WM_NULL;
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// Logging subclass, installed *before* muda's so it sees messages first.
unsafe extern "system" fn spy_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    match msg {
        WM_ENTERMENULOOP => log(format!(
            "WM_ENTERMENULOOP (modal menu loop starts; frames so far {})",
            FRAMES.load(Ordering::Relaxed)
        )),
        WM_EXITMENULOOP => log(format!(
            "WM_EXITMENULOOP (frames so far {})",
            FRAMES.load(Ordering::Relaxed)
        )),
        WM_INITMENUPOPUP => log("WM_INITMENUPOPUP"),
        WM_COMMAND => log(format!("WM_COMMAND id={} source={}", wparam & 0xFFFF, (wparam >> 16) & 0xFFFF)),
        WM_SIZE => log(format!("WM_SIZE client {}x{}", lparam & 0xFFFF, (lparam >> 16) & 0xFFFF)),
        _ => {}
    }
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

// ---------------------------------------------------------------- vizia side

#[derive(Debug)]
enum ProbeEvent {
    Init,
    /// A muda menu item id, delivered through ContextProxy.
    Menu(String),
}

struct Probe {
    installed: bool,
}

fn acc(s: &str) -> Option<Accelerator> {
    Some(s.parse().expect("accelerator"))
}

impl Probe {
    fn install(&mut self, cx: &mut EventContext) {
        // 1. The HWND: the fork's public `ModifyWindow::window()` returns the winit window of
        //    the current view's window (vizia_winit/src/lib.rs). No fork change needed.
        let Some(win) = cx.window() else {
            log("Init: root Window has no winit window yet");
            return;
        };
        let RawWindowHandle::Win32(h) = win.window_handle().unwrap().as_raw() else {
            unreachable!()
        };
        let hwnd = h.hwnd.get();
        HWND_MAIN.store(hwnd, Ordering::Relaxed);
        let before = client_rect(hwnd);
        let inner_before = win.inner_size();
        log(format!(
            "Init: HWND=0x{hwnd:X}; client {}x{} (winit inner {}x{}) before SetMenu",
            before.0, before.1, inner_before.width, inner_before.height
        ));

        unsafe { SetWindowSubclass(hwnd as HWND, Some(spy_proc), 900, 0) };

        // 2. The menu.
        let file = Submenu::with_items(
            "&File",
            true,
            &[
                &MenuItem::with_id("file.new", "&New", true, acc("CmdOrCtrl+N")),
                &MenuItem::with_id("file.open", "&Open...", true, acc("CmdOrCtrl+O")),
                &MenuItem::with_id("file.save", "&Save", true, acc("CmdOrCtrl+S")),
                &MenuItem::with_id("file.save_as", "Save &As...", true, acc("CmdOrCtrl+Shift+S")),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id("file.exit", "E&xit", true, None),
            ],
        )
        .unwrap();
        let edit = Submenu::with_items(
            "&Edit",
            true,
            &[
                &MenuItem::with_id("edit.undo", "&Undo", true, acc("CmdOrCtrl+Z")),
                &MenuItem::with_id("edit.redo", "&Redo", true, acc("CmdOrCtrl+Y")),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id("edit.duplicate", "&Duplicate", true, acc("CmdOrCtrl+D")),
            ],
        )
        .unwrap();
        let menu = Menu::with_items(&[&file, &edit]).unwrap();

        // 3. Menu events -> vizia: muda calls the handler synchronously from its window
        //    subclass (WM_COMMAND, UI thread). ContextProxy is Send but not Sync, hence Mutex.
        let proxy = Mutex::new(cx.get_proxy());
        MenuEvent::set_event_handler(Some(Arc::new(move |e: MenuEvent| {
            log(format!("muda handler: MenuEvent id={:?} -> ContextProxy::emit", e.id.0));
            let _ = proxy.lock().unwrap().emit(ProbeEvent::Menu(e.id.0.clone()));
        })));

        unsafe { menu.init_for_hwnd(hwnd).unwrap() };
        HACCEL.store(menu.haccel(), Ordering::Relaxed);
        let after = client_rect(hwnd);
        let inner_after = win.inner_size();
        log(format!(
            "Init: menu attached; haccel=0x{:X}; client {}x{} (winit inner {}x{}) after SetMenu",
            menu.haccel(),
            after.0,
            after.1,
            inner_after.width,
            inner_after.height
        ));

        if ACCEL_MODE.load(Ordering::Relaxed) > 0 {
            let hook = unsafe {
                SetWindowsHookExW(WH_GETMESSAGE, Some(getmsg_hook), std::ptr::null_mut(), GetCurrentThreadId())
            };
            log(format!("Init: WH_GETMESSAGE hook installed: {:?}", hook));
        }
        // The menu must outlive the window (muda's subclass keeps a pointer to it).
        Box::leak(Box::new(menu));
        self.installed = true;
    }
}

impl Model for Probe {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|e: &ProbeEvent, _| match e {
            ProbeEvent::Init => {
                if !self.installed {
                    self.install(cx);
                }
            }
            ProbeEvent::Menu(id) => {
                log(format!("vizia Model: ProbeEvent::Menu({id}) received"));
                if id == "file.exit" {
                    cx.emit(WindowEvent::WindowClose);
                }
            }
        });
        event.map(|we: &WindowEvent, _| match we {
            WindowEvent::KeyDown(code, _) => {
                let m = *cx.modifiers();
                log(format!("vizia KeyDown {code:?} ctrl={} shift={} alt={}", m.ctrl(), m.shift(), m.alt()));
                // The app's own shortcut path (crates/ui/src/lib.rs:643-670 does the same).
                if m.ctrl() {
                    let action = match code {
                        Code::KeyS => Some("Save"),
                        Code::KeyZ => Some("Undo"),
                        Code::KeyY => Some("Redo"),
                        Code::KeyN => Some("New"),
                        _ => None,
                    };
                    if let Some(a) = action {
                        log(format!("APP SHORTCUT fired via vizia KeyDown: {a}"));
                    }
                }
            }
            WindowEvent::GeometryChanged(_) => {}
            WindowEvent::WindowClose => log("vizia WindowClose"),
            _ => {}
        });
    }
}

/// Fills the window; redraws every ~16 ms from a vizia timer and logs stalls and its bounds.
struct Pulse {
    last_bounds: std::cell::Cell<(f32, f32, f32, f32)>,
}

impl Pulse {
    fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self { last_bounds: std::cell::Cell::new((0.0, 0.0, 0.0, 0.0)) }.build(cx, |cx| {
            let t = cx.add_timer(PUMP, None, |cx, action| {
                if let TimerAction::Tick(_) = action {
                    TICKS.fetch_add(1, Ordering::Relaxed);
                    let now = now_us();
                    let prev = LAST_TICK_US.swap(now, Ordering::Relaxed);
                    if prev != 0 && now - prev > STALL.as_micros() as u64 {
                        log(format!("timer STALL: {:.0} ms without a tick", (now - prev) as f64 / 1000.0));
                    }
                    cx.needs_redraw();
                }
            });
            cx.start_timer(t);
        })
    }
}

impl View for Pulse {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let n = FRAMES.fetch_add(1, Ordering::Relaxed) + 1;
        let now = now_us();
        let prev = LAST_DRAW_US.swap(now, Ordering::Relaxed);
        if prev != 0 && now - prev > STALL.as_micros() as u64 {
            log(format!("draw STALL: {:.0} ms without a frame (frame #{n})", (now - prev) as f64 / 1000.0));
        }
        if n % 120 == 0 {
            log(format!("frames={n} ticks={}", TICKS.load(Ordering::Relaxed)));
        }
        let b = cx.bounds();
        let key = (b.x, b.y, b.w, b.h);
        if key != self.last_bounds.get() {
            self.last_bounds.set(key);
            let hwnd = HWND_MAIN.load(Ordering::Relaxed);
            let cr = if hwnd != 0 { client_rect(hwnd) } else { (0, 0) };
            log(format!(
                "layout: Pulse bounds x={} y={} w={} h={} (physical px); client rect {}x{}",
                b.x, b.y, b.w, b.h, cr.0, cr.1
            ));
        }
        let rect: vg::Rect = b.into();
        let mut p = vg::Paint::default();
        p.set_anti_alias(true);
        p.set_color(vg::Color::from_rgb(22, 24, 31));
        canvas.draw_rect(rect, &p);
        // A bar sweeping left to right once a second: frozen bar = frozen rendering.
        let phase = (START.get().unwrap().elapsed().as_secs_f32()) % 1.0;
        let x = rect.left + phase * (rect.width() - 40.0);
        p.set_color(vg::Color::from_rgb(124, 92, 255));
        canvas.draw_rect(vg::Rect::from_xywh(x, rect.top + 20.0, 40.0, rect.height() - 40.0), &p);
        // Outline exactly on the bounds: all four edges must be visible in a screenshot.
        p.set_style(vg::PaintStyle::Stroke);
        p.set_stroke_width(4.0);
        p.set_color(vg::Color::from_rgb(0, 245, 160));
        canvas.draw_rect(rect.with_inset((2.0, 2.0)), &p);
    }
}

fn main() -> Result<(), ApplicationError> {
    START.get_or_init(Instant::now);
    let _ = LOG.set(Mutex::new(std::fs::File::create("menu_vizia.log").unwrap()));
    let args: Vec<String> = std::env::args().collect();
    let mode = args.iter().position(|a| a == "--accel").and_then(|i| args.get(i + 1)).map(String::as_str);
    ACCEL_MODE.store(
        match mode {
            Some("hook") => 1,
            Some("hook-swallow") => 2,
            _ => 0,
        },
        Ordering::Relaxed,
    );
    log(format!("start; accel mode {:?}", mode.unwrap_or("none")));

    Application::new(|cx| {
        Probe { installed: false }.build(cx);
        VStack::new(cx, |cx| {
            Label::new(cx, "TOP EDGE: menu probe (File / Edit above this line)").height(Pixels(24.0));
            Pulse::new(cx).width(Stretch(1.0)).height(Stretch(1.0));
            Label::new(cx, "BOTTOM EDGE").height(Pixels(24.0));
        });
        cx.emit(ProbeEvent::Init);
    })
    .title("MenuProbe")
    .inner_size((800, 500))
    .run()
}
