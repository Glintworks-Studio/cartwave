//! The app side of MIDI import as tracks (BATCH4B_PROMPT.md §6): the drag
//! preview, the drop, File ▸ Import ▸ MIDI File…, and the conflict dialog.
//! The import itself is `midi_tracks`.
//!
//! **Tracking the cursor during an OS file drag (VISION.md's "to verify").**
//! On Windows the file comes through OLE drag and drop: while it lasts the
//! window gets no mouse messages and winit reports only "a file entered"
//! (`HoveredFile`, forwarded by our vizia fork as `WindowEvent::FileHover`),
//! the drop and "it left". The preview still follows the cursor: between the
//! hover and the drop the app polls the OS cursor every frame
//! (`GetCursorPos` → client px). Verified on this machine with a scripted
//! OLE drag (docs/batch4b/tools/drag_midi.ps1, RUNLOG B4B-D16). Elsewhere
//! (macOS, untested here) there is no poll: the preview shows where an
//! import lands without a position (after the last track) until the drop.

use std::path::{Path, PathBuf};

use vizia::prelude::*;

use crate::midi_tracks::{self, Conflict, Mode, PlannedPart, Target};
use crate::state::{SoundChip, TICKS_PER_BAR};
use crate::warn::{Choice, WarnKind};
use crate::AppState;

/// One part in the drag preview.
#[derive(Clone, Debug, PartialEq)]
pub struct Ghost {
    pub name: String,
    pub count: usize,
    pub first: u32,
    pub len: u32,
    /// What it would break under the profile ("chords", "no free channel"…);
    /// empty when it fits.
    pub warn: String,
}

/// What the arrangement draws while a `.mid` is dragged over the window.
#[derive(Clone, Debug, PartialEq)]
pub struct DragView {
    pub ghosts: Vec<Ghost>,
    /// The cursor (window client px, physical), when the platform lets us
    /// track it.
    pub pos: Option<(f32, f32)>,
}

pub fn is_midi(p: &Path) -> bool {
    p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "mid" | "midi"))
}

/// The OS cursor in the window's client px (Windows; None elsewhere).
pub fn cursor_client(hwnd: Option<isize>) -> Option<(f32, f32)> {
    #[cfg(all(windows, feature = "menus"))]
    {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
        use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
        let hwnd = hwnd?;
        let mut p = POINT { x: 0, y: 0 };
        // SAFETY: plain Win32 calls with a valid out pointer and our own window.
        let ok = unsafe { GetCursorPos(&mut p) != 0 && ScreenToClient(hwnd as _, &mut p) != 0 };
        ok.then_some((p.x as f32, p.y as f32))
    }
    #[cfg(not(all(windows, feature = "menus")))]
    {
        let _ = hwnd;
        None
    }
}

/// Where a drop at `pos` (client px) lands: the track under it (onto) or
/// the end, and the bar under it (ticks, snapped down to the bar).
pub fn target_at(a: &crate::state::Arrangement, pos: Option<(f32, f32)>) -> (Target, u32) {
    let Some((x, y)) = pos else { return (Target::End, 0) };
    let lane = crate::views::browser::lane_at(x, y);
    let tick = match crate::views::arrangement::arrangement_at() {
        Some((left, right, s)) if x >= left && x < right => {
            let t = a.view.x_to_tick(((x - left) / s) as f64).max(0.0) as u32;
            t / TICKS_PER_BAR * TICKS_PER_BAR
        }
        _ => 0,
    };
    (lane.map_or(Target::End, Target::Onto), tick)
}

fn warn_text(c: &[Conflict]) -> String {
    c.iter().map(|x| x.text()).collect::<Vec<_>>().join(", ")
}

impl AppState {
    fn midi_plan_for(&mut self, path: &Path) -> Result<Vec<PlannedPart>, String> {
        if let Some((p, plan)) = &self.midi_plan {
            if p == path {
                return Ok(plan.clone());
            }
        }
        let bytes = std::fs::read(path).map_err(|e| format!("Can't open {}: {e}", path.display()))?;
        let song = crate::midi::parse(&bytes)?;
        let plan = midi_tracks::plan(&song);
        if plan.is_empty() {
            return Err(format!("{} has no notes.", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
        }
        self.midi_plan = Some((path.to_path_buf(), plan.clone()));
        Ok(plan)
    }

    /// A file entered the window (an OS drag).
    pub(crate) fn midi_hover(&mut self, path: &std::path::Path) {
        if !is_midi(path) {
            return;
        }
        let Ok(plan) = self.midi_plan_for(path) else { return };
        let chip = self.s.chip.get_untracked();
        let conflicts = self.s.arrangement.with_untracked(|a| midi_tracks::conflicts(&plan, a, chip));
        let ghosts = plan
            .iter()
            .zip(conflicts)
            .map(|(p, c)| Ghost { name: p.name.clone(), count: p.count(), first: p.first, len: p.len, warn: warn_text(&c) })
            .collect();
        if crate::dev::enabled() {
            eprintln!("dev: midi hover {} ({} parts)", path.display(), plan.len());
        }
        self.s.midi_drag.set(Some(DragView { ghosts, pos: cursor_client(self.hwnd) }));
    }

    pub(crate) fn midi_hover_end(&mut self) {
        if self.s.midi_drag.with_untracked(|d| d.is_some()) {
            self.s.midi_drag.set(None);
        }
    }

    /// Every frame while a drag is over the window: the preview follows the
    /// cursor.
    pub(crate) fn midi_drag_tick(&mut self) {
        if self.s.midi_drag.with_untracked(|d| d.is_none()) {
            return;
        }
        let pos = cursor_client(self.hwnd);
        self.s.midi_drag.update(|d| {
            if let Some(d) = d {
                if d.pos != pos {
                    d.pos = pos;
                }
            }
        });
        if crate::dev::enabled() {
            if let Some((x, y)) = pos {
                let t = self.s.arrangement.with_untracked(|a| target_at(a, Some((x, y))));
                if self.last_drag_log != Some(t) {
                    self.last_drag_log = Some(t);
                    eprintln!("dev: midi drag at {x:.0},{y:.0} -> {:?} bar {}", t.0, t.1 / TICKS_PER_BAR + 1);
                }
            }
        }
    }

    /// A `.mid` dropped on the window: tracks added where it landed.
    pub(crate) fn midi_drop(&mut self, path: &Path) {
        let pos = cursor_client(self.hwnd).or_else(|| self.s.midi_drag.with_untracked(|d| d.as_ref().and_then(|d| d.pos)));
        self.midi_hover_end();
        let (target, at) = self.s.arrangement.with_untracked(|a| target_at(a, pos));
        self.import_midi_parts(path, target, at);
    }

    /// File ▸ Import ▸ MIDI File…: new tracks after the last one, at the
    /// insert marker's bar.
    pub(crate) fn import_midi_menu(&mut self) {
        // `CARTWAVE_IMPORT_FROM=file.mid` (dev, scripted UI tests): no native dialog.
        let path = match crate::dev::var_os("CARTWAVE_IMPORT_FROM") {
            Some(p) => PathBuf::from(p),
            None => match rfd::FileDialog::new().add_filter("MIDI file", &["mid", "midi"]).pick_file() {
                Some(p) => p,
                None => return,
            },
        };
        let at = self.s.arrangement.with_untracked(|a| a.insert.0 / TICKS_PER_BAR * TICKS_PER_BAR);
        self.import_midi_parts(&path, Target::End, at);
    }

    /// Adds `path`'s parts as tracks; asks only if something conflicts with
    /// the hardware profile (VISION principle 6). One undo step.
    pub(crate) fn import_midi_parts(&mut self, path: &Path, target: Target, at: u32) {
        let plan = match self.midi_plan_for(path) {
            Ok(p) => p,
            Err(e) => return self.set_status(e),
        };
        self.midi_plan = None;
        let chip = self.s.chip.get_untracked();
        let conflicts = self.s.arrangement.with_untracked(|a| midi_tracks::conflicts(&plan, a, chip));
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mode = if midi_tracks::any_conflict(&conflicts) {
            let lines: Vec<String> = plan.iter().zip(&conflicts).filter(|(_, c)| !c.is_empty()).map(|(p, c)| format!("• {}: {}", p.name, warn_text(c))).collect();
            let text = format!(
                "{} parts of {name} don't fit {}:\n\n{}\n\nImport them as studio tracks to play everything as written, or fit them to the hardware: free channels, notes folded into range, chords reduced (or played as fast arpeggios).",
                lines.len(),
                chip.label(),
                lines.join("\n")
            );
            let q = crate::warn::Ask {
                title: "Import this MIDI file?",
                text: &text,
                unlock: Some("Import as studio tracks (recommended)"),
                fit: Some("Fit to hardware"),
                fit_arp: Some("Fit to hardware, chords as arpeggios"),
                default: Some(Choice::Unlock),
            };
            match self.warn(WarnKind::MidiImport, &q) {
                Choice::Unlock => Mode::Studio,
                Choice::Fit => Mode::Fit { arp: false },
                Choice::FitArp => Mode::Fit { arp: true },
                Choice::Cancel => return self.set_status(format!("Import of {name} cancelled")),
            }
        } else if chip == SoundChip::StudioOnly {
            Mode::Studio
        } else {
            // Everything fits the profile: locked tracks, no question.
            Mode::Fit { arp: false }
        };
        let bpm = self.s.transport.with_untracked(|t| t.bpm);
        // Each new track's colour: automatic (distinct from the last tracks
        // and the earlier picks), or the default colour (Settings).
        let auto = self.settings.auto_track_colours;
        let reduced = self.settings.reduced_auto_colours;
        let fixed = self.new_track_color_at(usize::MAX);
        let mut all: Vec<crate::theme::tokens::Rgba> = self.s.arrangement.with_untracked(|a| a.lanes.iter().map(|l| l.color).collect());
        let mut pick = move |_| {
            if !auto {
                return fixed;
            }
            let lo = all.len().saturating_sub(2);
            let c = crate::theme::palette::next_track_color(&all, &all[lo..], reduced);
            all.push(c);
            c
        };
        let mut out = (String::new(), Vec::new());
        self.s.arrangement.update(|a| out = midi_tracks::import(a, &plan, at, target, mode, chip, bpm, &mut pick));
        // Fit: whatever the hardware still can't play on the new locked
        // tracks (a DPCM row with no sample loaded) is fitted too, as the
        // profile warnings' Fit does, so the commit has nothing to ask.
        if let Mode::Fit { .. } = mode {
            let mine: Vec<crate::track_ui::Conflict> = self.s.arrangement.with_untracked(|a| crate::track_ui::conflicts(a, chip)).into_iter().filter(|c| out.1.contains(&c.track)).collect();
            for kind in [WarnKind::Chord, WarnKind::Range, WarnKind::Dpcm] {
                let these: Vec<crate::track_ui::Conflict> = mine.iter().copied().filter(|c| c.kind == crate::track_ui::kind_code(kind)).collect();
                if !these.is_empty() {
                    self.fit_conflicts(kind, &these);
                }
            }
        }
        // The song grows to hold the new clips.
        let end = self.s.arrangement.with_untracked(|a| a.clips.iter().map(|c| c.start_ticks + c.len_ticks).max().unwrap_or(0));
        let x = self.extras();
        if end > x.loop_end {
            self.set_song_loop(x.loop_start as f64, end as f64, false);
        }
        self.sync_tracks();
        self.commit();
        if crate::dev::enabled() {
            eprintln!("dev: midi import {name}: {:?} at bar {} {:?}: {}", target, at / TICKS_PER_BAR + 1, mode, out.0);
        }
        self.set_status_for(format!("Imported {name} as {} tracks: {}", out.1.len(), out.0), crate::REPORT_SECS);
    }
}
