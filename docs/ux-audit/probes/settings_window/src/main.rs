//! Probe: a separate Settings window with ChipDAW's vizia fork (winit backend).
//!
//! The main window ("SettingsProbe") has an "Open Settings" button and an animated strip
//! that redraws every ~16 ms; the Settings window ("Settings") has a slider and a checkbox
//! bound to the same model signals as labels in the main window. Each window's strip counts
//! its own frames; once a second the log prints frames per window, so the cost of a second
//! window on the main window's frame rate is visible.
//!
//! Run: `settings_window`; log: `settings_window.log` in the cwd.

#![windows_subsystem = "windows"]

use std::cell::Cell;
use std::io::Write;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use vizia::prelude::*;
use vizia::vg;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

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

/// Redraw pump (about one 60 Hz frame).
const PUMP: Duration = Duration::from_millis(16);

thread_local! {
    /// Frames drawn in the current second: [main, settings].
    static FRAMES: Cell<[u32; 2]> = const { Cell::new([0, 0]) };
    static WINDOW_START: Cell<Option<Instant>> = const { Cell::new(None) };
}

struct AppData {
    show_settings: Signal<bool>,
    volume: Signal<f32>,
    dark: Signal<bool>,
}

enum AppEvent {
    OpenSettings,
    SettingsClosed,
    SetVolume(f32),
    ToggleDark,
}

impl Model for AppData {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|e: &AppEvent, meta| {
            // `origin` tells which window's view sent it.
            let from = meta.origin;
            match e {
                AppEvent::OpenSettings => {
                    log(format!("AppEvent::OpenSettings (from {from:?})"));
                    self.show_settings.set(true);
                }
                AppEvent::SettingsClosed => {
                    log("AppEvent::SettingsClosed (Window::on_close)");
                    self.show_settings.set(false);
                }
                AppEvent::SetVolume(v) => {
                    log(format!("AppEvent::SetVolume({v:.2}) from {from:?}, handled by the main window's model"));
                    self.volume.set(*v);
                }
                AppEvent::ToggleDark => {
                    self.dark.update(|d| *d = !*d);
                    log(format!("AppEvent::ToggleDark -> {} from {from:?}", self.dark.get()));
                }
            }
        });
        event.map(|we: &WindowEvent, meta| match we {
            WindowEvent::MouseDown(b) => log(format!("WindowEvent::MouseDown({b:?}) target {:?}", meta.target)),
            WindowEvent::KeyDown(c, _) => log(format!("WindowEvent::KeyDown({c:?}) target {:?}", meta.target)),
            WindowEvent::WindowClose => log(format!("WindowEvent::WindowClose target {:?}", meta.target)),
            WindowEvent::WindowFocused(f) => log(format!("WindowEvent::WindowFocused({f}) target {:?}", meta.target)),
            _ => {}
        });
    }
}

/// An animated strip; `slot` 0 = main window, 1 = settings window.
struct Strip {
    slot: usize,
}

impl Strip {
    fn new(cx: &mut Context, slot: usize) -> Handle<'_, Self> {
        Self { slot }.build(cx, |cx| {
            let t = cx.add_timer(PUMP, None, |cx, action| {
                if let TimerAction::Tick(_) = action {
                    cx.needs_redraw();
                }
            });
            cx.start_timer(t);
        })
    }
}

impl View for Strip {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        FRAMES.with(|f| {
            let mut v = f.get();
            v[self.slot] += 1;
            f.set(v);
        });
        WINDOW_START.with(|s| {
            let now = Instant::now();
            let start = s.get().unwrap_or(now);
            if s.get().is_none() {
                s.set(Some(now));
            } else if now - start >= Duration::from_secs(1) {
                let v = FRAMES.with(|f| f.replace([0, 0]));
                log(format!("frames in the last {:.2} s: main={} settings={}", (now - start).as_secs_f32(), v[0], v[1]));
                s.set(Some(now));
            }
        });
        let b = cx.bounds();
        let rect: vg::Rect = b.into();
        let mut p = vg::Paint::default();
        p.set_color(vg::Color::from_rgb(22, 24, 31));
        canvas.draw_rect(rect, &p);
        let phase = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 1000) as f32
            / 1000.0;
        p.set_color(vg::Color::from_rgb(124, 92, 255));
        canvas.draw_rect(
            vg::Rect::from_xywh(rect.left + phase * (rect.width() - 30.0), rect.top, 30.0, rect.height()),
            &p,
        );
    }
}

fn main() -> Result<(), ApplicationError> {
    let _ = LOG.set(Mutex::new(std::fs::File::create("settings_window.log").unwrap()));
    log("start");
    Application::new(|cx| {
        let show_settings = Signal::new(false);
        let volume = Signal::new(0.5f32);
        let dark = Signal::new(true);
        AppData { show_settings, volume, dark }.build(cx);

        // The settings window exists while `show_settings` is true (vizia's multiwindow
        // example pattern). Its content closure runs in the main Context, so it binds to
        // the same signals and its events bubble up to AppData.
        Binding::new(cx, show_settings, move |cx| {
            if show_settings.get() {
                Window::new(cx, move |cx| {
                    VStack::new(cx, |cx| {
                        Label::new(cx, "Settings");
                        Slider::new(cx, volume).on_change(|cx, v| cx.emit(AppEvent::SetVolume(v)));
                        HStack::new(cx, |cx| {
                            Checkbox::new(cx, dark).on_toggle(|cx| cx.emit(AppEvent::ToggleDark));
                            Label::new(cx, "Dark theme");
                        })
                        .height(Auto)
                        .horizontal_gap(Pixels(8.0));
                        Strip::new(cx, 1).height(Pixels(20.0));
                    })
                    .padding(Pixels(20.0))
                    .vertical_gap(Pixels(12.0));
                })
                .on_create(|_cx| log("Settings window on_create (OS window exists)"))
                .on_close(|cx| cx.emit(AppEvent::SettingsClosed))
                .title("Settings")
                .inner_size((400, 220))
                .anchor(Anchor::Center);
            }
        });

        VStack::new(cx, |cx| {
            Button::new(cx, |cx| Label::new(cx, "Open Settings"))
                .on_press(|cx| cx.emit(AppEvent::OpenSettings));
            Label::new(cx, volume.map(|v| format!("volume (shared model) = {v:.2}")));
            Label::new(cx, dark.map(|d| format!("dark (shared model) = {d}")));
            Strip::new(cx, 0).height(Pixels(40.0));
        })
        .padding(Pixels(12.0))
        .vertical_gap(Pixels(8.0));
    })
    .title("SettingsProbe")
    .inner_size((600, 360))
    .position((1600, 200))
    .run()
}
