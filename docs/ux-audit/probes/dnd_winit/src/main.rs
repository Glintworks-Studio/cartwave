//! Probe: which winit 0.30.13 events arrive while a file from another process is dragged
//! over the window (OLE drag), and whether any of them carry a position.
//!
//! Every `WindowEvent` is logged with wall-clock time. While a file hovers (between
//! `HoveredFile` and `DroppedFile` / `HoveredFileCancelled`) the loop also wakes every 16 ms
//! and polls `GetCursorPos` (the "poll while hovered" workaround), logged as `poll`.
//!
//! Run: `dnd_winit` (window title "DndWinitProbe"); log: `dnd_winit.log` in the cwd.

#![windows_subsystem = "windows"]

use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::{Window, WindowId};

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
    println!("{line}");
    if let Some(f) = LOG.get() {
        let _ = writeln!(f.lock().unwrap(), "{line}");
    }
}

/// How often the cursor is polled while a file hovers.
const POLL: Duration = Duration::from_millis(16);

#[derive(Default)]
struct App {
    window: Option<Window>,
    hwnd: isize,
    hovering: bool,
    last_poll: Option<(i32, i32)>,
}

impl App {
    fn poll_cursor(&mut self) {
        let mut p = POINT { x: 0, y: 0 };
        unsafe {
            GetCursorPos(&mut p);
            ScreenToClient(self.hwnd as HWND, &mut p);
        }
        if self.last_poll != Some((p.x, p.y)) {
            self.last_poll = Some((p.x, p.y));
            log(format!("poll (GetCursorPos while hovered): client ({}, {})", p.x, p.y));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_none() {
            let w = el
                .create_window(
                    Window::default_attributes()
                        .with_title("DndWinitProbe")
                        .with_inner_size(winit::dpi::PhysicalSize::new(700, 450))
                        // Right half of a 2560 px screen, clear of a running ChipDAW.
                        .with_position(winit::dpi::PhysicalPosition::new(1600, 200)),
                )
                .unwrap();
            let RawWindowHandle::Win32(h) = w.window_handle().unwrap().as_raw() else {
                unreachable!()
            };
            self.hwnd = h.hwnd.get();
            log(format!("window created, HWND=0x{:X}", self.hwnd));
            self.window = Some(w);
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match &event {
            WindowEvent::RedrawRequested | WindowEvent::Moved(_) => return,
            WindowEvent::CloseRequested => {
                log("CloseRequested");
                el.exit();
                return;
            }
            WindowEvent::HoveredFile(_) => self.hovering = true,
            WindowEvent::DroppedFile(_) | WindowEvent::HoveredFileCancelled => {
                self.hovering = false;
                self.last_poll = None;
            }
            _ => {}
        }
        let text = match &event {
            WindowEvent::CursorMoved { position, .. } => {
                format!("CursorMoved ({:.0}, {:.0})", position.x, position.y)
            }
            WindowEvent::KeyboardInput { event, .. } => {
                format!("KeyboardInput {:?} {:?}", event.physical_key, event.state)
            }
            e => format!("{e:?}"),
        };
        log(format!("WindowEvent::{text}"));
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        if self.hovering {
            self.poll_cursor();
            el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + POLL));
        } else {
            el.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() {
    let _ = LOG.set(Mutex::new(std::fs::File::create("dnd_winit.log").unwrap()));
    let el = EventLoop::new().unwrap();
    el.run_app(&mut App::default()).unwrap();
}
