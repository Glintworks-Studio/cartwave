//! In-app modal panels (UX.md §2–3, FEASIBILITY.md §3): Settings, Project
//! Settings, About and the keyboard-shortcut sheet. One at a time, over a
//! backdrop that dims the window and takes every click; Esc or Close
//! dismisses it, and no other shortcut reaches the document while it is open
//! (`shortcuts::Ctx::Modal`). Panels rather than OS windows, so they work the
//! same in a plugin window.

use vizia::prelude::*;

use crate::file::Settings;
use crate::notes::Grid;
use crate::state::{Snap, SoundChip};
use crate::{AppEvent, Signals};
use engine::FilterMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    Audio,
    Display,
    Editing,
    Files,
    Warnings,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 5] = [Self::Audio, Self::Display, Self::Editing, Self::Files, Self::Warnings];

    fn label(self) -> &'static str {
        match self {
            Self::Audio => "Audio",
            Self::Display => "Display",
            Self::Editing => "Editing",
            Self::Files => "Files",
            Self::Warnings => "Warnings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modal {
    Settings(SettingsTab),
    ProjectSettings,
    About,
    Shortcuts,
}

/// One change made in the Settings panel.
#[derive(Clone, Debug, PartialEq)]
pub enum Set {
    Device(Option<String>),
    Buffer(Option<u32>),
    Rate(u32),
    FrameCap(u32),
    ClockBars(bool),
    ArrangementGrid(String),
    PianoGrid(String),
    Follow(bool),
    Autosave(u32),
    OfferRecovery(bool),
    OpenLast(bool),
    Unsilence(String),
    UnsilenceAll,
}

/// The output filter choice in Project Settings (`None`: follow the profile).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilterPick(pub Option<FilterMode>);

/// Buffer sizes offered (frames); `None` is the device's default.
pub const BUFFERS: [Option<u32>; 6] = [None, Some(128), Some(256), Some(512), Some(1024), Some(2048)];
pub const RATES: [u32; 3] = [44_100, 48_000, 96_000];
pub const AUTOSAVE: [(u32, &str); 5] = [(0, "Off"), (60, "1 min"), (120, "2 min"), (300, "5 min"), (600, "10 min")];

/// Panel width (logical px), on the 4 px grid.
const PANEL_W: f32 = 640.0;
/// The shortcut sheet's scrolling height.
const SHEET_H: f32 = 480.0;

fn row(cx: &mut Context, label: &str, content: impl FnOnce(&mut Context)) {
    HStack::new(cx, |cx| {
        Label::new(cx, label.to_string()).class("modal-label");
        HStack::new(cx, content).class("modal-choices");
    })
    .class("modal-row");
}

fn choice(cx: &mut Context, text: impl Into<String>, on: bool, enabled: bool, ev: impl Fn() -> AppEvent + Send + Sync + 'static) {
    let text = text.into();
    Button::new(cx, move |cx| Label::new(cx, text.clone()))
        .class("btn")
        .toggle_class("checked", on)
        .disabled(!enabled)
        .on_press(move |cx| cx.emit(ev()));
}

fn note(cx: &mut Context, text: impl Into<String>) {
    Label::new(cx, text.into()).class("modal-note");
}

fn set(v: Set) -> impl Fn() -> AppEvent + Send + Sync + 'static {
    move || AppEvent::SetSetting(v.clone())
}

pub fn build(cx: &mut Context, s: Signals) {
    let open = Memo::new(move |_| s.modal.get().is_some());
    Binding::new(cx, open, move |cx| {
        if !open.get() {
            return;
        }
        Backdrop::new(cx).class("modal-backdrop").position_type(PositionType::Absolute).width(Stretch(1.0)).height(Stretch(1.0));
        Binding::new(cx, s.modal, move |cx| {
            let Some(m) = s.modal.get() else { return };
            VStack::new(cx, |cx| {
                let title = match m {
                    Modal::Settings(_) => "SETTINGS",
                    Modal::ProjectSettings => "PROJECT SETTINGS",
                    Modal::About => "ABOUT CHIPDAW",
                    Modal::Shortcuts => "KEYBOARD SHORTCUTS",
                };
                HStack::new(cx, |cx| {
                    Label::new(cx, title).class("section-title");
                    Element::new(cx).width(Stretch(1.0)).height(Pixels(1.0));
                    Button::new(cx, |cx| Label::new(cx, "Close")).class("btn").on_press(|cx| cx.emit(AppEvent::CloseModal));
                })
                .class("modal-head");
                match m {
                    Modal::Settings(tab) => settings(cx, s, tab),
                    Modal::ProjectSettings => project_settings(cx, s),
                    Modal::About => about(cx),
                    Modal::Shortcuts => sheet(cx),
                }
            })
            .class("modal")
            .width(Pixels(PANEL_W))
            .position_type(PositionType::Absolute)
            .left(Stretch(1.0))
            .right(Stretch(1.0))
            .top(Pixels(crate::theme::tokens::TOPBAR_HEIGHT + 24.0));
        });
    });
}

fn settings(cx: &mut Context, s: Signals, tab: SettingsTab) {
    HStack::new(cx, |cx| {
        for t in SettingsTab::ALL {
            choice(cx, t.label(), t == tab, true, move || AppEvent::OpenModal(Modal::Settings(t)));
        }
    })
    .class("modal-tabs");
    Element::new(cx).class("modal-rule");
    Binding::new(cx, s.settings, move |cx| {
        let st = s.settings.get();
        match tab {
            SettingsTab::Audio => audio(cx, s, &st),
            SettingsTab::Display => display(cx, &st),
            SettingsTab::Editing => editing(cx, &st),
            SettingsTab::Files => files(cx, &st),
            SettingsTab::Warnings => warnings(cx, &st),
        }
    });
}

fn audio(cx: &mut Context, s: Signals, st: &Settings) {
    let rate = st.sample_rate;
    Binding::new(cx, s.audio_devices, {
        let dev = st.audio_device.clone();
        move |cx| {
            let devices = s.audio_devices.get();
            row(cx, "Output device", |cx| {
                VStack::new(cx, |cx| {
                    choice(cx, "System default", dev.is_none(), true, set(Set::Device(None)));
                    for d in devices {
                        let on = dev.as_deref() == Some(d.as_str());
                        choice(cx, d.clone(), on, true, set(Set::Device(Some(d))));
                    }
                })
                .class("modal-list");
            });
        }
    });
    row(cx, "Buffer size", |cx| {
        for b in BUFFERS {
            let text = match b {
                None => "Device default".to_string(),
                // Latency of one buffer at the chosen rate.
                Some(n) => format!("{n} · {:.1} ms", n as f32 * 1000.0 / rate as f32),
            };
            choice(cx, text, st.buffer_frames == b, true, set(Set::Buffer(b)));
        }
    });
    row(cx, "Sample rate", |cx| {
        for r in RATES {
            let text = if r % 1000 == 0 { format!("{} kHz", r / 1000) } else { format!("{:.1} kHz", r as f32 / 1000.0) };
            choice(cx, text, st.sample_rate == r, true, set(Set::Rate(r)));
        }
    });
    Label::new(cx, s.audio_info).class("modal-note");
}

fn display(cx: &mut Context, st: &Settings) {
    row(cx, "Frame-rate cap", |cx| {
        choice(cx, "60 fps", st.frame_cap == 60, true, set(Set::FrameCap(60)));
        choice(cx, "120 fps", st.frame_cap == 120, true, set(Set::FrameCap(120)));
        if crate::dev::enabled() {
            choice(cx, "Off (dev)", st.frame_cap == 0, true, set(Set::FrameCap(0)));
        }
    });
    note(cx, "60 fps uses about half the power of 120 with the most even motion; 120 is smoother on fast displays.");
    row(cx, "Interface scale", |cx| {
        choice(cx, "Follows the system", true, false, || AppEvent::CloseModal);
    });
    note(cx, "The interface follows the system's display scaling. An in-app scale needs a change to the UI toolkit and comes in a later build.");
    row(cx, "Clock shows", |cx| {
        choice(cx, "Time (m:ss.cc)", !st.clock_bars, true, set(Set::ClockBars(false)));
        choice(cx, "Bars (bar.beat.16th)", st.clock_bars, true, set(Set::ClockBars(true)));
    });
}

fn editing(cx: &mut Context, st: &Settings) {
    row(cx, "Arrangement grid", |cx| {
        for g in [Snap::Sixteenth, Snap::Quarter, Snap::Bar] {
            choice(cx, g.label(), st.arrangement_grid == g.label(), true, set(Set::ArrangementGrid(g.label().into())));
        }
    });
    row(cx, "Piano-roll grid", |cx| {
        for g in Grid::ALL {
            choice(cx, g.label(), st.piano_grid == g.label(), true, set(Set::PianoGrid(g.label().into())));
        }
    });
    note(cx, "The grid new projects and newly opened clips start with.");
    row(cx, "Follow playhead", |cx| {
        choice(cx, "On", st.follow_playhead, true, set(Set::Follow(true)));
        choice(cx, "Off", !st.follow_playhead, true, set(Set::Follow(false)));
    });
}

fn files(cx: &mut Context, st: &Settings) {
    row(cx, "Autosave", |cx| {
        for (secs, text) in AUTOSAVE {
            choice(cx, text, st.autosave_secs == secs, true, set(Set::Autosave(secs)));
        }
    });
    note(cx, "Autosave writes a recovery copy, never over your project file.");
    row(cx, "Offer recovery after an unsaved quit", |cx| {
        choice(cx, "On", st.offer_recovery, true, set(Set::OfferRecovery(true)));
        choice(cx, "Off", !st.offer_recovery, true, set(Set::OfferRecovery(false)));
    });
    row(cx, "Open last project at start", |cx| {
        choice(cx, "On", st.open_last, true, set(Set::OpenLast(true)));
        choice(cx, "Off", !st.open_last, true, set(Set::OpenLast(false)));
    });
    let dir = crate::file::data_dir().map(|d| d.display().to_string()).unwrap_or_else(|| "(none)".into());
    note(cx, format!("Settings, recovery and presets live in {dir}"));
}

fn warnings(cx: &mut Context, st: &Settings) {
    if st.silenced.is_empty() {
        note(cx, "Every warning is on: nothing was turned off with \"Don't ask again\".");
    }
    for id in &st.silenced {
        let id = id.clone();
        row(cx, &id.clone(), |cx| {
            choice(cx, "Reset", false, true, set(Set::Unsilence(id)));
        });
    }
    row(cx, "", |cx| {
        choice(cx, "Reset all warnings", false, !st.silenced.is_empty(), set(Set::UnsilenceAll));
    });
}

fn project_settings(cx: &mut Context, s: Signals) {
    Binding::new(cx, s.chip, move |cx| {
        let chip = s.chip.get();
        row(cx, "Hardware profile", |cx| {
            choice(cx, "2A03", chip == SoundChip::Base2A03, true, || AppEvent::SetProfile(SoundChip::Base2A03));
            choice(cx, "2A03 + VRC6", chip == SoundChip::WithVrc6, true, || AppEvent::SetProfile(SoundChip::WithVrc6));
        });
        note(cx, "The console the song is written for. With 2A03 alone, the three VRC6 lanes are silent and dimmed.");
    });
    Binding::new(cx, s.filter_state, move |cx| {
        let (mode, auto) = s.filter_state.get();
        row(cx, "Output filter", |cx| {
            let follows = format!("Follow the profile ({})", FilterMode::default_for(s.chip.get_untracked() == SoundChip::WithVrc6).label());
            choice(cx, follows, auto, true, || AppEvent::SetFilter(FilterPick(None)));
            for m in FilterMode::ALL {
                choice(cx, m.label(), !auto && m == mode, true, move || AppEvent::SetFilter(FilterPick(Some(m))));
            }
        });
    });
    note(cx, "Saved in the project. Tempo lives in the top bar.");
}

/// The licences and credits shown in About (from docs/ASSETS.md).
pub const CREDITS: &[&str] = &[
    "Licence: MIT OR Apache-2.0.",
    "Fonts: Inter 4.001, © 2016 The Inter Project Authors, and JetBrains Mono 2.304, © 2020 The JetBrains Mono Project Authors; both SIL Open Font License 1.1, embedded unmodified.",
    "Sounds, presets, the demo songs and the built-in drum samples are this project's own work, made by the code and scripts in the repository (docs/ASSETS.md). Nothing is taken from game ROMs.",
    "\"2A03\", \"VRC6\", \"NES\" and \"Famicom\" name the hardware this app emulates.",
];

fn about(cx: &mut Context) {
    Label::new(cx, format!("ChipDAW {}  ·  build {}", env!("CARGO_PKG_VERSION"), env!("CHIPDAW_COMMIT"))).class("modal-title");
    for line in CREDITS {
        note(cx, *line);
    }
}

fn sheet(cx: &mut Context) {
    ScrollView::new(cx, |cx| {
        VStack::new(cx, |cx| {
            for (group, rows) in crate::shortcuts::sheet(crate::shortcuts::MACOS) {
                Label::new(cx, group.to_uppercase()).class("field-label").class("sheet-group");
                for (name, keys) in rows {
                    HStack::new(cx, |cx| {
                        Label::new(cx, name).class("sheet-name");
                        Label::new(cx, keys).class("sheet-keys");
                    })
                    .class("sheet-row");
                }
            }
        })
        .height(Auto)
        .class("sheet");
    })
    .height(Pixels(SHEET_H));
}

/// Dims the window and takes every click while a panel is open.
pub struct Backdrop;

impl Backdrop {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {})
    }
}

impl View for Backdrop {
    fn element(&self) -> Option<&'static str> {
        Some("modal-backdrop")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(_) => {
                cx.emit(AppEvent::CloseModal);
                meta.consume();
            }
            WindowEvent::MouseScroll(..) | WindowEvent::MouseUp(_) | WindowEvent::MouseDoubleClick(_) => meta.consume(),
            _ => {}
        });
    }
}
