//! The export dialog (STABILIZE_PROMPT.md §C3): range, rate, bit depth,
//! loop files, stems and loudness, with progress and cancel. Functional
//! styling: stock `.btn` toggles in labelled rows.

use vizia::prelude::*;
use crate::HelpExt;

use crate::game_export::{Loudness, Range, Stems};
use crate::{AppEvent, Signals};

/// One choice in the dialog.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pick {
    RangeSong,
    RangeLoop,
    RangeSelection,
    Rate(u32),
    Bits(u16),
    Single,
    Split,
    Json,
    StemsNone,
    StemsChannels,
    StemsGroups,
    LoudAuthentic,
    LoudPeak,
    LoudLufs,
}

/// The dialog's state: what to export and how the job is going.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportDialog {
    pub opts: crate::game_export::Options,
    /// Each lane's group (0 melody, 1 bass, 2 drums, 3 none) for group stems.
    pub group_of: [u8; 8],
    /// A time selection exists (the Selection range is available).
    pub has_selection: bool,
    /// The job: progress in permille while running.
    pub running: Option<u32>,
    /// The last result (files written, or the error).
    pub result: String,
}

pub const GROUP_NAMES: [&str; 4] = ["melody", "bass", "drums", "—"];
/// Lane names short enough for the group chips (click a chip to move it on).
const SHORT_LANE: [&str; 8] = ["Pulse 1", "Pulse 2", "Triangle", "Noise", "DPCM", "VRC6 P1", "VRC6 P2", "Saw"];

impl ExportDialog {
    pub fn new(has_selection: bool) -> Self {
        let mut group_of = [3u8; 8];
        for (gi, g) in crate::game_export::default_groups().iter().enumerate() {
            for (lane, slot) in group_of.iter_mut().enumerate() {
                if g.lanes & (1 << lane) != 0 {
                    *slot = gi as u8;
                }
            }
        }
        Self { opts: Default::default(), group_of, has_selection, running: None, result: String::new() }
    }

    /// The group stems the lane assignment describes.
    pub fn groups(&self) -> Vec<crate::game_export::Group> {
        (0..3)
            .map(|g| crate::game_export::Group {
                name: GROUP_NAMES[g].into(),
                lanes: (0..8).filter(|&l| self.group_of[l] == g as u8).fold(0u8, |m, l| m | 1 << l),
            })
            .filter(|g| g.lanes != 0)
            .collect()
    }

    pub fn picked(&self, p: Pick) -> bool {
        let o = &self.opts;
        match p {
            Pick::RangeSong => o.range == Range::Song,
            Pick::RangeLoop => o.range == Range::Loop,
            Pick::RangeSelection => matches!(o.range, Range::Selection { .. }),
            Pick::Rate(r) => o.rate == r,
            Pick::Bits(b) => o.bits == b,
            Pick::Single => o.single,
            Pick::Split => o.split,
            Pick::Json => o.json,
            Pick::StemsNone => o.stems == Stems::None,
            Pick::StemsChannels => o.stems == Stems::Channels,
            Pick::StemsGroups => matches!(o.stems, Stems::Groups(_)),
            Pick::LoudAuthentic => o.loudness == Loudness::Authentic,
            Pick::LoudPeak => matches!(o.loudness, Loudness::Peak { .. }),
            Pick::LoudLufs => matches!(o.loudness, Loudness::Lufs { .. }),
        }
    }

    /// Applies a click. `sel` is the current time selection (ticks).
    pub fn pick(&mut self, p: Pick, sel: Option<(u32, u32)>) {
        let o = &mut self.opts;
        match p {
            Pick::RangeSong => o.range = Range::Song,
            Pick::RangeLoop => o.range = Range::Loop,
            Pick::RangeSelection => {
                if let Some((start, end)) = sel {
                    o.range = Range::Selection { start, end };
                }
            }
            Pick::Rate(r) => o.rate = r,
            Pick::Bits(b) => o.bits = b,
            // At least one loop file kind stays on.
            Pick::Single => o.single = !o.single || !o.split,
            Pick::Split => o.split = !o.split || !o.single,
            Pick::Json => o.json = !o.json,
            Pick::StemsNone => o.stems = Stems::None,
            Pick::StemsChannels => o.stems = Stems::Channels,
            Pick::StemsGroups => o.stems = Stems::Groups(Vec::new()),
            Pick::LoudAuthentic => o.loudness = Loudness::Authentic,
            Pick::LoudPeak => o.loudness = Loudness::Peak { dbfs: -1.0 },
            Pick::LoudLufs => o.loudness = Loudness::Lufs { lufs: -16.0, ceiling_dbtp: -1.0 },
        }
    }

    /// The choices as settings.json keeps them (E4).
    pub fn remembered(&self) -> Vec<String> {
        let o = &self.opts;
        let range = match o.range {
            Range::Song => "song",
            Range::Loop => "loop",
            Range::Selection { .. } => "selection",
        };
        let stems = match o.stems {
            Stems::None => "none",
            Stems::Channels => "channels",
            Stems::Groups(_) => "groups",
        };
        let loud = match o.loudness {
            Loudness::Authentic => "authentic",
            Loudness::Peak { .. } => "peak",
            Loudness::Lufs { .. } => "lufs",
        };
        vec![
            format!("range:{range}"),
            format!("rate:{}", o.rate),
            format!("bits:{}", o.bits),
            format!("single:{}", o.single as u8),
            format!("split:{}", o.split as u8),
            format!("json:{}", o.json as u8),
            format!("stems:{stems}"),
            format!("loud:{loud}"),
            format!("groups:{}", self.group_of.iter().map(|g| char::from(b'0' + g.min(&3))).collect::<String>()),
        ]
    }

    /// Applies remembered choices; unknown or unavailable ones are skipped
    /// (a Selection range needs a selection now).
    pub fn restore(&mut self, keys: &[String], sel: Option<(u32, u32)>) {
        for k in keys {
            let Some((key, v)) = k.split_once(':') else { continue };
            let pick = match (key, v) {
                ("range", "song") => Some(Pick::RangeSong),
                ("range", "loop") => Some(Pick::RangeLoop),
                ("range", "selection") if sel.is_some() => Some(Pick::RangeSelection),
                ("rate", r) => r.parse().ok().filter(|r| [44_100, 48_000].contains(r)).map(Pick::Rate),
                ("bits", b) => b.parse().ok().filter(|b| [16, 24].contains(b)).map(Pick::Bits),
                ("stems", "none") => Some(Pick::StemsNone),
                ("stems", "channels") => Some(Pick::StemsChannels),
                ("stems", "groups") => Some(Pick::StemsGroups),
                ("loud", "authentic") => Some(Pick::LoudAuthentic),
                ("loud", "peak") => Some(Pick::LoudPeak),
                ("loud", "lufs") => Some(Pick::LoudLufs),
                _ => None,
            };
            if let Some(p) = pick {
                self.pick(p, sel);
            }
            match (key, v) {
                ("single", b) => self.opts.single = b == "1",
                ("split", b) => self.opts.split = b == "1",
                ("json", b) => self.opts.json = b == "1",
                ("groups", g) if g.len() == 8 => {
                    for (lane, c) in g.bytes().enumerate() {
                        self.group_of[lane] = c.saturating_sub(b'0').min(3);
                    }
                }
                _ => {}
            }
        }
        // At least one loop file kind stays on.
        if !self.opts.single && !self.opts.split {
            self.opts.single = true;
        }
    }

    /// The options to run, with the group stems filled in.
    pub fn options(&self) -> crate::game_export::Options {
        let mut o = self.opts.clone();
        if let Stems::Groups(_) = o.stems {
            o.stems = Stems::Groups(self.groups());
        }
        o
    }
}

fn row(cx: &mut Context, label: &str, content: impl FnOnce(&mut Context)) {
    HStack::new(cx, |cx| {
        Label::new(cx, label.to_string()).class("export-label");
        HStack::new(cx, content).gap(Pixels(4.0)).height(Auto).width(Auto);
    })
    .class("export-row");
}

/// A choice; a disabled one says why on hover (E3).
fn choice(cx: &mut Context, d: &ExportDialog, text: &str, p: Pick, enabled: bool) {
    let on = d.picked(p);
    let text = text.to_string();
    let why: Option<&'static str> = if d.running.is_some() {
        Some("Wait for the export to finish (or cancel it).")
    } else if !enabled && p == Pick::RangeSelection {
        Some("Make a time selection first: drag across empty lane space in the arrangement.")
    } else if !enabled {
        Some("Loop files are made only for the Loop region range.")
    } else {
        None
    };
    let button = move |cx: &mut Context| {
        Button::new(cx, move |cx| Label::new(cx, text.clone()))
            .class("btn")
            .toggle_class("checked", on)
            .disabled(!enabled)
            .on_press(move |cx| cx.emit(AppEvent::ExportPick(p)));
    };
    match why {
        // On a wrapper: a disabled button is never hovered.
        Some(why) => {
            crate::tip_wrap(cx, button).help(why);
        }
        None => button(cx),
    }
}

pub fn build(cx: &mut Context, s: Signals) {
    let open = Memo::new(move |_| s.export_dlg.get().is_some());
    Binding::new(cx, open, move |cx| {
        if !open.get() {
            return;
        }
        super::modal::Backdrop::new(cx, AppEvent::ExportClose)
            .class("modal-backdrop")
            .position_type(PositionType::Absolute)
            .width(Stretch(1.0))
            .height(Stretch(1.0));
        Binding::new(cx, s.export_dlg, move |cx| {
            let Some(d) = s.export_dlg.get() else { return };
            let busy = d.running.is_some();
            let looping = d.opts.range == Range::Loop;
            VStack::new(cx, |cx| {
                Label::new(cx, "EXPORT").class("section-title");
                row(cx, "Range", |cx| {
                    choice(cx, &d, "Song", Pick::RangeSong, !busy);
                    choice(cx, &d, "Loop region", Pick::RangeLoop, !busy);
                    choice(cx, &d, "Selection", Pick::RangeSelection, !busy && d.has_selection);
                });
                row(cx, "Format", |cx| {
                    choice(cx, &d, "44.1 kHz", Pick::Rate(44_100), !busy);
                    choice(cx, &d, "48 kHz", Pick::Rate(48_000), !busy);
                    choice(cx, &d, "16-bit", Pick::Bits(16), !busy);
                    choice(cx, &d, "24-bit", Pick::Bits(24), !busy);
                });
                row(cx, "Loop files", |cx| {
                    choice(cx, &d, "One WAV + loop points", Pick::Single, !busy && looping);
                    choice(cx, &d, "Intro + loop", Pick::Split, !busy && looping);
                    choice(cx, &d, "JSON", Pick::Json, !busy && looping);
                });
                row(cx, "Stems", |cx| {
                    choice(cx, &d, "Mix only", Pick::StemsNone, !busy);
                    choice(cx, &d, "8 channels", Pick::StemsChannels, !busy);
                    choice(cx, &d, "Groups", Pick::StemsGroups, !busy);
                });
                if matches!(d.opts.stems, Stems::Groups(_)) {
                    // Two rows of four, short lane names: eight chips don't fit in one.
                    for (label, lanes) in [("Groups", 0..4), ("", 4..8)] {
                        row(cx, label, |cx| {
                            for lane in lanes {
                                let t = format!("{} · {}", SHORT_LANE[lane], GROUP_NAMES[d.group_of[lane] as usize]);
                                Button::new(cx, move |cx| Label::new(cx, t.clone()))
                                    .class("btn")
                                    .disabled(busy)
                                    .on_press(move |cx| cx.emit(AppEvent::ExportGroup(lane)));
                            }
                        });
                    }
                }
                row(cx, "Loudness", |cx| {
                    choice(cx, &d, "Authentic chip level", Pick::LoudAuthentic, !busy);
                    choice(cx, &d, "Peak −1 dBFS", Pick::LoudPeak, !busy);
                    choice(cx, &d, "−16 LUFS (≤ −1 dBTP)", Pick::LoudLufs, !busy);
                });
                let status = match d.running {
                    Some(pm) => format!("Rendering… {}%", pm / 10),
                    None => d.result.clone(),
                };
                Label::new(cx, status).class("export-status");
                HStack::new(cx, |cx| {
                    if busy {
                        Button::new(cx, |cx| Label::new(cx, "Cancel")).class("btn").on_press(|cx| cx.emit(AppEvent::ExportCancel));
                    } else {
                        Button::new(cx, |cx| Label::new(cx, "Export…"))
                            .class("btn")
                            .class("checked")
                            .on_press(|cx| cx.emit(AppEvent::ExportGo))
                            .help("Choose where to save, then render (Enter).");
                        Button::new(cx, |cx| Label::new(cx, "Close"))
                            .class("btn")
                            .on_press(|cx| cx.emit(AppEvent::ExportClose))
                            .help("Close (Esc). Your choices are kept for next time.");
                    }
                })
                .gap(Pixels(8.0))
                .height(Auto);
            })
            .class("export-dialog")
            .position_type(PositionType::Absolute)
            .left(Stretch(1.0))
            .right(Stretch(1.0))
            .top(Pixels(crate::theme::tokens::TOPBAR_HEIGHT + 24.0));
        });
    });
}
