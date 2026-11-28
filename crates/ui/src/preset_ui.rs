//! Presets in the app (PRESETS_PROMPT.md §4–§6; VISION.md "Instruments",
//! "Presets and the browser", "Hardware profiles and locked tracks"):
//! loading a preset onto a track (a preset remembers its voice), the lock
//! rules (a studio-only preset on a locked track asks Unlock · Cancel; an
//! authentic preset locks the track and sets the profile, asking first
//! where it would break the budget), ◀ ▶ stepping through a folder (one
//! coalesced undo step), the Voice selector, and creating a track from a
//! drop on empty space. Each command is one undo step.

use chip::voice::VoiceKind;
use vizia::prelude::*;

use crate::state::SoundChip;
use crate::user_presets::PresetView;
use crate::warn::{Choice, WarnKind};
use crate::{bank, tracks, AppState};

/// Puts a preset (factory or user) on track `lane`: its instrument and
/// channel controls; on a studio track its voice (when it doesn't suit the
/// track's) and its Unison / Detune / Drift.
pub fn apply_view(a: &mut crate::state::Arrangement, lane: usize, p: &PresetView) {
    let Some(t) = a.lanes.get(lane) else { return };
    let studio = t.lock.is_none();
    if studio && !p.fits(t.voice.lane()) {
        a.set_voice(lane, p.voice);
    }
    a.instruments.set_lane(lane, p.inst.clone());
    let clips = a.clips.clone();
    a.instruments.prune(&clips);
    if let Some(l) = a.lanes.get_mut(lane) {
        let chan = l.chan();
        let mut row = engine::Params::default();
        row.values[chan] = l.params;
        for &(id, v) in &p.params {
            if chip::driver::ParamId::for_lane(chan).contains(&id) {
                row.set(chan, id, v as i32);
            }
        }
        l.params = row.values[chan];
        if studio && !p.user {
            l.unison = p.unison;
        }
    }
}

/// How a load commits.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Load {
    /// Its own undo step.
    Commit,
    /// The caller commits (◀ ▶ coalesce, a new track and its preset).
    Deferred,
}

impl AppState {
    /// The preset (factory or user index) a track's instrument came from.
    pub(crate) fn current_preset(&self, lane: usize) -> Option<usize> {
        let name = self.s.arrangement.with_untracked(|a| a.instruments.lane(lane).map(|i| i.name.clone()))?;
        if let Some(i) = bank::index_of(&name) {
            return Some(i);
        }
        self.s.user_presets.with_untracked(|u| u.iter().position(|p| p.name.eq_ignore_ascii_case(name.trim_end_matches('*')))).map(|i| bank::len() + i)
    }

    /// Whether a preset loads on track `lane` as it is: a studio track takes
    /// anything (it switches voice); a locked track takes a preset of its
    /// channel's type with no studio-only control.
    pub(crate) fn preset_fits(&self, p: &PresetView, lane: usize) -> bool {
        self.s.arrangement.with_untracked(|a| {
            a.lanes.get(lane).is_some_and(|l| match l.lock {
                None => true,
                Some(c) => p.fits(c) && !p.studio_only(),
            })
        })
    }

    /// The track a preset goes to: `want`, else the selected track, else
    /// the first one (a studio track takes any preset).
    pub(crate) fn preset_target(&self, want: Option<usize>) -> Option<usize> {
        let n = self.s.arrangement.with_untracked(|a| a.lanes.len());
        let sel = self.s.lane_sel.get_untracked();
        want.filter(|l| *l < n).or((sel < n).then_some(sel)).or((n > 0).then_some(0))
    }

    /// AssignPreset: loads preset `p` on a track (`None`: the selected one).
    pub(crate) fn assign_preset(&mut self, cx: &mut EventContext, lane: Option<usize>, p: usize) {
        let Some(lane) = self.preset_target(lane) else { return };
        if self.s.arrangement.with_untracked(|a| crate::state::lane_off(self.s.chip.get_untracked(), a, lane)) {
            return self.set_status(crate::state::VRC6_OFF_HINT.into());
        }
        self.preset_step = None;
        self.load_preset(cx, lane, p, Load::Commit);
    }

    /// Loads preset `p` on track `lane`, asking first where it matters.
    /// Returns whether it loaded.
    pub(crate) fn load_preset(&mut self, cx: &mut EventContext, lane: usize, p: usize, how: Load) -> bool {
        let Some(pr) = self.preset(p) else { return false };
        let loaded = if pr.profile.is_some() { self.load_authentic(cx, lane, &pr) } else { self.load_studio(lane, &pr) };
        if !loaded {
            return false;
        }
        self.sync_tracks();
        if lane == self.s.lane_sel.get_untracked() {
            self.s.dials.set(self.dial_values(lane));
        }
        if how == Load::Commit {
            self.commit();
        }
        let (name, lock) = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| (l.name.clone(), l.lock)).unwrap_or_default());
        self.set_status(match lock {
            Some(c) if pr.profile.is_some() => format!("{} loaded on \"{name}\", locked to the {}", pr.name, tracks::channel_name(c)),
            _ => format!("{} loaded on \"{name}\"", pr.name),
        });
        if crate::dev::enabled() {
            let (voice, lock, inst) = self.s.arrangement.with_untracked(|a| (a.lanes[lane].voice, a.lanes[lane].lock, a.instruments.lane(lane).map(|i| i.name.clone())));
            eprintln!("dev: preset {p} \"{}\" on track {lane}: voice {voice:?} lock {lock:?} instrument {inst:?} profile {:?}", pr.name, self.s.chip.get_untracked());
        }
        true
    }

    /// A studio (non-authentic) preset. On a locked track that can't play it
    /// (another voice, or Unison / Detune / Drift) it asks: Unlock track ·
    /// Cancel (VISION.md principle 5).
    fn load_studio(&mut self, lane: usize, pr: &PresetView) -> bool {
        let lock = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).and_then(|l| l.lock));
        if let Some(c) = lock {
            if !pr.fits(c) || pr.studio_only() {
                let name = self.s.arrangement.with_untracked(|a| a.lanes[lane].name.clone());
                let why = if !pr.fits(c) {
                    format!("plays on the {} voice", bank::voice_name(pr.voice))
                } else {
                    "uses Unison, Detune or Drift, studio-only controls".to_string()
                };
                let text = format!(
                    "\"{}\" {why}: the {} can't play it as it is.\n\nUnlock \"{name}\" to load it: the track becomes a studio track (polyphony and every control). Or cancel and choose an authentic preset for this channel.",
                    pr.name,
                    tracks::channel_name(c)
                );
                let q = crate::warn::Ask { title: "A studio preset on a locked track", text: &text, unlock: Some("Unlock track"), fit: None, fit_arp: None, default: None };
                match self.warn(WarnKind::StudioPreset, &q) {
                    Choice::Unlock => self.s.arrangement.update(|a| a.unlock_track(lane)),
                    _ => return false,
                }
            }
        }
        self.s.arrangement.update(|a| apply_view(a, lane, pr));
        true
    }

    /// An authentic preset (Hardware): the project goes under its profile
    /// and the track is locked to a free channel of its type (VISION.md).
    /// Asks first if that would change the hardware (another expansion
    /// chip), if no channel of the type is free, or (4a's rules) if the
    /// track's fader or pan isn't neutral or it has chords.
    fn load_authentic(&mut self, cx: &mut EventContext, lane: usize, pr: &PresetView) -> bool {
        let want = pr.profile.unwrap_or(SoundChip::Base2A03);
        let chans = tracks::channels_of(pr.voice);
        let cur = self.s.chip.get_untracked();
        let (lock, voice, tname) = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| (l.lock, l.voice, l.name.clone()))).unwrap_or((None, pr.voice, String::new()));
        // A track locked to another type of channel can't take it.
        if let Some(c) = lock {
            if !chans.contains(&c) {
                self.set_status(format!("{} is for the {} channel; \"{tname}\" is locked to the {}. Drop it on empty space for a new track, or on a {} track.", pr.name, bank::voice_name(pr.voice), tracks::channel_name(c), bank::voice_name(pr.voice)));
                return false;
            }
        }
        let _ = voice;
        // The profile.
        if !chans.iter().any(|&c| cur.has_channel(c)) {
            if cur != SoundChip::StudioOnly {
                // Another chip setup (§5.5): switch, load unlocked, or cancel.
                let text = format!(
                    "\"{}\" is an authentic preset for {}. This project's hardware profile is {}.\n\nSwitch the profile to {} and lock the track to its channel, or load the preset on the track without a lock.",
                    pr.name,
                    want.label(),
                    cur.label(),
                    want.label()
                );
                let fit = format!("Switch to {}", want.label());
                let q = crate::warn::Ask { title: "A preset for another chip setup", text: &text, unlock: Some("Load unlocked"), fit: Some(&fit), fit_arp: None, default: None };
                match self.warn(WarnKind::Expansion, &q) {
                    Choice::Fit => {
                        if !self.profile_switch_ok(want) {
                            return false;
                        }
                    }
                    Choice::Unlock => return self.load_unlocked(lane, pr),
                    Choice::FitArp | Choice::Cancel => return false,
                }
            }
            // None (studio only) has nothing to break: the project simply
            // goes under the preset's profile (VISION.md "Hardware profiles").
            self.set_chip(want);
        }
        let chip = self.s.chip.get_untracked();
        // The channel: the track's own if it's already locked to one of the
        // type, else a free one.
        let chan = match lock {
            Some(c) => c,
            None => {
                let free = self.s.arrangement.with_untracked(|a| chans.iter().copied().find(|&c| chip.has_channel(c) && a.channel_owner(c).is_none()));
                match free {
                    Some(c) => c,
                    None => {
                        let owners: Vec<String> = self.s.arrangement.with_untracked(|a| chans.iter().filter_map(|&c| a.channel_owner(c).map(|o| format!("\"{}\" ({})", a.lanes[o].name, tracks::channel_name(c)))).collect());
                        let text = format!(
                            "\"{}\" locks its track to a {} channel, and every one the profile has is taken:\n\n{}\n\nLoad it on \"{tname}\" without a lock (a studio track), or cancel and unlock one of those tracks first.",
                            pr.name,
                            bank::voice_name(pr.voice),
                            owners.join("\n")
                        );
                        let q = crate::warn::Ask { title: "No free channel", text: &text, unlock: Some("Load unlocked"), fit: None, fit_arp: None, default: None };
                        return match self.warn(WarnKind::Budget, &q) {
                            Choice::Unlock => self.load_unlocked(lane, pr),
                            _ => false,
                        };
                    }
                }
            }
        };
        if lock.is_none() {
            // 4a's lock rules: the fader and pan reset, chords fitted.
            if self.s.arrangement.with_untracked(|a| a.lock_resets(lane)) {
                let (db, pan) = self.s.arrangement.with_untracked(|a| (a.lanes[lane].vol_db, a.lanes[lane].pan));
                let text = crate::track_ui::lock_reset_text(&tname, db, pan);
                if self.ask(cx, &text, "Reset and lock", None) != Some(true) {
                    return false;
                }
            }
            if !self.check_mono_for_lock(cx, lane) {
                return false;
            }
            self.s.arrangement.update(|a| {
                a.set_voice(lane, VoiceKind::of_lane(chan));
                a.lock_track(lane, chan);
            });
        }
        self.s.arrangement.update(|a| apply_view(a, lane, pr));
        true
    }

    /// An authentic preset loaded as a studio preset (no lock): on a locked
    /// track of another channel the track unlocks first.
    fn load_unlocked(&mut self, lane: usize, pr: &PresetView) -> bool {
        self.s.arrangement.update(|a| {
            if a.lanes.get(lane).and_then(|l| l.lock).is_some_and(|c| !pr.fits(c)) {
                a.unlock_track(lane);
            }
            apply_view(a, lane, pr);
        });
        true
    }

    /// ◀ ▶ (§6.2): the previous / next preset in the current one's folder,
    /// loaded at once, even while playing. A run of steps is one undo step.
    /// On a locked track, presets it can't play as they are are skipped.
    pub(crate) fn step_preset(&mut self, cx: &mut EventContext, dir: i32) {
        let lane = self.s.lane_sel.get_untracked();
        let Some(cur) = self.current_preset(lane) else {
            return self.set_status("This instrument isn't from a preset folder: choose one from the preset field.".into());
        };
        let list: Vec<usize> = if cur >= bank::len() {
            (0..self.s.user_presets.with_untracked(|u| u.len())).map(|i| bank::len() + i).collect()
        } else {
            bank::siblings(cur)
        };
        let n = list.len() as i32;
        let Some(pos) = list.iter().position(|&p| p == cur) else { return };
        let locked = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).is_some_and(|l| l.lock.is_some()));
        let next = (1..n).map(|k| list[((pos as i32 + dir * k).rem_euclid(n)) as usize]).find(|&p| {
            !locked || self.preset(p).is_some_and(|v| self.preset_fits(&v, lane))
        });
        let Some(next) = next else {
            return self.set_status("No other preset in this folder plays on this locked track.".into());
        };
        // One undo step for a run of steps on the same track: the first
        // commits, the others fold into it while nothing else was committed.
        let run = self.preset_step.filter(|(l, steps)| *l == lane && *steps == self.history.steps()).is_some();
        if !run {
            // Anything pending is its own step first.
            self.commit();
        }
        if !self.load_preset(cx, lane, next, Load::Deferred) {
            return;
        }
        let d = self.doc();
        if run {
            self.history.amend(d);
        } else {
            self.history.commit(d);
        }
        self.preset_step = Some((lane, self.history.steps()));
        self.update_dirty();
        // Stopped: a short preview, as selecting in the browser does.
        if !self.s.transport.with_untracked(|t| t.playing) {
            self.audition(Some(next));
            self.audition_until = self.audition_lane.map(|_| std::time::Instant::now() + crate::AUDITION_BRIEF);
        }
        if crate::dev::enabled() {
            eprintln!("dev: step preset {dir:+} -> {next} undo steps {:?}", self.history.steps());
        }
    }

    /// The Voice selector (§4.2): a studio track plays another voice; one
    /// undo step. A locked track's voice is its channel's.
    pub(crate) fn set_voice_cmd(&mut self, lane: usize, k: VoiceKind) {
        let mut changed = false;
        self.s.arrangement.update(|a| changed = a.set_voice(lane, k));
        if !changed {
            return;
        }
        self.preset_step = None;
        self.sync_tracks();
        if lane == self.s.lane_sel.get_untracked() {
            self.s.dials.set(self.dial_values(lane));
        }
        self.commit();
        self.set_status(format!("The track plays the {} voice now.", bank::voice_name(k)));
        if crate::dev::enabled() {
            eprintln!("dev: voice track {lane} -> {k:?} undo steps {:?}", self.history.steps());
        }
    }

    /// A preset dropped on empty space (§5.3): a new track of its voice at
    /// the end with the preset on it (an authentic one locks it); one undo
    /// step. Cancelling a question leaves no track behind.
    pub(crate) fn preset_new_track(&mut self, cx: &mut EventContext, p: usize) {
        let Some(pr) = self.preset(p) else { return };
        self.commit();
        let color = self.next_track_color();
        let mut at = 0;
        self.s.arrangement.update(|a| {
            at = a.insert_track(a.lanes.len(), pr.voice, color);
            a.track_sel = vec![at];
            a.selection.clear();
        });
        self.s.lane_sel.set(at);
        self.preset_step = None;
        if !self.load_preset(cx, at, p, Load::Deferred) {
            self.s.arrangement.update(|a| {
                a.delete_track(at);
                a.track_sel.clear();
            });
            self.s.lane_sel.set(self.s.arrangement.with_untracked(|a| a.lanes.len().saturating_sub(1)));
            self.sync_tracks();
            return;
        }
        self.commit();
        let name = self.s.arrangement.with_untracked(|a| a.lanes[at].name.clone());
        self.set_status(format!("New track \"{name}\" with {}", pr.name));
        if crate::dev::enabled() {
            eprintln!("dev: new track {at} from preset {p} undo steps {:?}", self.history.steps());
        }
    }

    /// A clip from the Clips folder: on track `lane` at `tick` (the insert
    /// marker if `None`), or (no track) on a new track of its role's voice.
    pub(crate) fn insert_phrase(&mut self, phrase: usize, lane: Option<usize>, tick: Option<u32>) {
        let Some(ph) = bank::PHRASES.get(phrase) else { return };
        self.commit();
        let voice = match ph.role {
            "bass" => VoiceKind::Triangle,
            "kit" | "hit" => VoiceKind::Noise,
            "dpcm_map" => VoiceKind::Dpcm,
            _ => VoiceKind::Pulse,
        };
        let lane = match lane {
            Some(l) => l,
            None => {
                let color = self.next_track_color();
                let mut at = 0;
                self.s.arrangement.update(|a| at = a.insert_track(a.lanes.len(), voice, color));
                at
            }
        };
        let tick = tick.unwrap_or_else(|| self.s.arrangement.with_untracked(|a| a.insert.0));
        let len = ph.bars * crate::state::TICKS_PER_BAR;
        let notes = bank::phrase_notes(ph, 0);
        let mut id = 0;
        let random = self.settings.new_clip_colour == "random";
        self.s.arrangement.update(|a| {
            id = a.add_clip(lane, tick, len);
            if let Some(c) = a.clip_mut(id) {
                c.name = ph.name.to_string();
                c.notes = notes;
                c.next_note_id = c.notes.len() as u32 + 1;
            }
            a.resolve_overlaps(&[id]);
            a.colour_new_clips(&[id], random);
            a.selection = vec![id];
        });
        self.commit();
        self.set_status(format!("Added the clip \"{}\"", ph.name));
    }

    /// A DPCM sample from the Samples folder onto a DPCM Sampler track
    /// (`None`: a new one, as a drop on empty space): every note plays it.
    pub(crate) fn assign_sample(&mut self, slot: usize, lane: Option<usize>) {
        self.commit();
        let lane = match lane {
            Some(l) => l,
            None => {
                let color = self.next_track_color();
                let mut at = 0;
                self.s.arrangement.update(|a| at = a.insert_track(a.lanes.len(), VoiceKind::Dpcm, color));
                at
            }
        };
        let ok = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).is_some_and(|l| l.voice == VoiceKind::Dpcm));
        if !ok {
            return self.set_status("Samples play on a DPCM Sampler track: drop it on one, or on empty space for a new one.".into());
        }
        self.s.arrangement.update(|a| {
            if let Some(l) = a.lanes.get_mut(lane) {
                l.params[chip::driver::ParamId::Sample as usize] = slot as i8;
            }
        });
        self.sync_tracks();
        self.commit();
        let name = chip::samples::NAMES.get(slot).copied().unwrap_or("the sample");
        self.set_status(format!("{name} on the DPCM Sampler"));
    }
}

