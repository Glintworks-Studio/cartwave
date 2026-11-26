//! Track commands in the app (BATCH4A_PROMPT.md §4.3, §5.2): Insert,
//! Duplicate, Delete, reorder, lock and unlock. Each is one undo step.

use chip::voice::VoiceKind;
use vizia::prelude::*;

use crate::state::{MenuAction, MenuItem};
use crate::warn::{Choice, WarnKind};
use crate::{tracks, AppState};

/// Locking: what the conflict dialog says (§5.2).
pub fn lock_reset_text(name: &str, db: f32, pan: i8) -> String {
    let what = match (db != 0.0, pan != 0) {
        (true, true) => format!("This track's fader is at {} and its pan at {}.", crate::state::format_db(db), crate::state::format_pan(pan)),
        (true, false) => format!("This track's fader is at {}.", crate::state::format_db(db)),
        _ => format!("This track's pan is at {}.", crate::state::format_pan(pan)),
    };
    format!("Lock \"{name}\" to the chip?\n\n{what} Locking resets it to 0 dB and centre: a locked track's level comes from the chip itself. Unlocking puts them back.")
}

impl AppState {
    /// The track index commands act on: the first selected track, else the
    /// selected clip's, else the insert marker's.
    pub(crate) fn current_track(&self) -> usize {
        self.s.arrangement.with_untracked(|a| {
            a.track_sel.first().copied().or_else(|| a.first_selected().map(|c| c.lane)).unwrap_or(a.insert.1).min(a.lanes.len().saturating_sub(1))
        })
    }

    /// Insert Track: a new studio track of `kind` (default: the current
    /// track's type) below the current track, coloured by the Track and Clip
    /// Colors settings; it becomes the selected track.
    pub(crate) fn insert_track(&mut self, kind: Option<VoiceKind>) {
        let cur = self.current_track();
        let kind = kind.unwrap_or_else(|| self.s.arrangement.with_untracked(|a| a.lanes.get(cur).map_or(VoiceKind::Pulse, |l| l.voice)));
        let empty = self.s.arrangement.with_untracked(|a| a.lanes.is_empty());
        let at = if empty { 0 } else { cur + 1 };
        let color = self.new_track_color_at(at);
        let mut i = 0;
        self.s.arrangement.update(|a| {
            i = a.insert_track(at, kind, color);
            a.track_sel = vec![i];
            a.selection.clear();
            a.insert.1 = i;
        });
        self.s.lane_sel.set(i);
        self.s.dials.set(self.dial_values(i));
        self.commit();
        let name = self.s.arrangement.with_untracked(|a| a.lanes[i].name.clone());
        self.set_status(format!("Inserted the studio track \"{name}\"."));
        if crate::dev::enabled() {
            eprintln!("dev: insert track {i} {kind:?} {name}");
        }
    }

    /// The tracks Duplicate / Delete Track act on: the selected tracks, else
    /// the current one.
    fn target_tracks(&self) -> Vec<usize> {
        let mut v = self.s.arrangement.with_untracked(|a| a.track_sel.clone());
        if v.is_empty() {
            v.push(self.current_track());
        }
        v.sort_unstable();
        v.dedup();
        v
    }

    pub(crate) fn duplicate_tracks(&mut self, only: Option<usize>) {
        let targets = only.map_or_else(|| self.target_tracks(), |t| vec![t]);
        let mut new = Vec::new();
        self.s.arrangement.update(|a| {
            // From the bottom up, so the indices above stay valid.
            for &t in targets.iter().rev() {
                if let Some(i) = a.duplicate_track(t) {
                    new.push(i);
                }
            }
            // The copies, in their final positions.
            let n = new.len();
            a.track_sel = new.iter().enumerate().map(|(k, &i)| i + (n - 1 - k)).collect();
            a.selection.clear();
        });
        self.commit();
        self.set_status(format!("Duplicated {} track(s); a copy of a locked track is a studio track.", targets.len()));
    }

    pub(crate) fn delete_tracks(&mut self, only: Option<usize>) {
        let targets = only.map_or_else(|| self.target_tracks(), |t| vec![t]);
        let n = self.s.arrangement.with_untracked(|a| a.lanes.len());
        if targets.len() >= n {
            return self.set_status("A project keeps at least one track.".into());
        }
        let ids: Vec<u32> = self.s.arrangement.with_untracked(|a| targets.iter().filter_map(|&t| a.lanes.get(t).map(|l| l.id)).collect());
        self.s.arrangement.update(|a| {
            for &t in targets.iter().rev() {
                a.delete_track(t);
            }
            let clips = a.clips.clone();
            a.instruments.prune(&clips);
        });
        self.s.mute.update(|m| ids.iter().for_each(|id| m.set(*id, false)));
        self.s.solo.update(|m| ids.iter().for_each(|id| m.set(*id, false)));
        let lane = self.s.lane_sel.get_untracked().min(n - 1 - targets.len());
        self.s.lane_sel.set(lane);
        self.s.dials.set(self.dial_values(lane));
        if self.s.piano.with_untracked(|p| p.clip).is_some_and(|c| self.s.arrangement.with_untracked(|a| a.clip(c).is_none())) {
            self.close_piano();
        }
        self.commit();
        self.set_status(format!("Deleted {} track(s).", targets.len()));
    }

    /// A header dragged from `from` to `to` (T4, R88).
    pub(crate) fn move_track(&mut self, from: usize, to: usize) {
        let mut moved = false;
        self.s.arrangement.update(|a| moved = a.move_track(from, to));
        if moved {
            self.s.lane_sel.set(to);
            self.s.dials.set(self.dial_values(to));
            self.commit();
            if crate::dev::enabled() {
                eprintln!("dev: move track {from} -> {to}");
            }
        }
    }

    /// The badge's menu (§5.2): Studio (unlocked), then the profile's
    /// channels of the track's type; a channel another track holds is
    /// disabled and names its owner.
    pub(crate) fn lock_menu_items(&self, lane: usize) -> Vec<MenuItem> {
        let profile = self.s.chip.get_untracked();
        self.s.arrangement.with_untracked(|a| {
            let Some(t) = a.lanes.get(lane) else { return Vec::new() };
            let mut v = vec![MenuItem::new(MenuAction::LockTrack(lane, None), "Studio (unlocked)").checked(t.lock.is_none())];
            v.push(MenuItem::sep());
            let chans: Vec<usize> = tracks::channels_of(t.voice).iter().copied().filter(|&c| profile.has_channel(c)).collect();
            if chans.is_empty() {
                let why = if profile == crate::state::SoundChip::StudioOnly {
                    "The project has no hardware profile: pick one in File > Project Settings… to lock tracks.".to_string()
                } else {
                    format!("{} has no {} channel.", profile.label(), tracks::kind_name(t.voice))
                };
                v.push(MenuItem::new(MenuAction::Info, "No channel to lock to").off_if(true, &why));
            }
            for c in chans {
                let owner = a.channel_owner(c).filter(|&o| o != lane);
                let label = match owner {
                    Some(o) => format!("{}  ({})", tracks::channel_name(c), a.lanes[o].name),
                    None => tracks::channel_name(c).to_string(),
                };
                let mut it = MenuItem::new(MenuAction::LockTrack(lane, Some(c)), label).checked(t.lock == Some(c));
                if let Some(o) = owner {
                    it = it.off_if(true, &format!("Taken by \"{}\": unlock that track first.", a.lanes[o].name));
                }
                v.push(it);
            }
            v
        })
    }

    /// Locks track `lane` to channel `chan` (`None`: unlocks it). Locking a
    /// track whose fader or pan isn't at 0 dB / centre asks first ("Reset and
    /// lock" / Cancel); chords on it are a profile conflict and ask too. One
    /// undo step restores the lock state and the old values.
    pub(crate) fn lock_track_cmd(&mut self, cx: &mut EventContext, lane: usize, chan: Option<usize>) {
        let Some((name, db, pan, lock, voice)) = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| (l.name.clone(), l.vol_db, l.pan, l.lock, l.voice))) else { return };
        match chan {
            None => {
                if lock.is_none() {
                    return;
                }
                self.s.arrangement.update(|a| a.unlock_track(lane));
                self.commit();
                self.set_status(format!("\"{name}\" is a studio track now: it sounds the same, with polyphony."));
            }
            Some(c) => {
                if lock == Some(c) || VoiceKind::of_lane(c) != voice || !self.s.chip.get_untracked().has_channel(c) {
                    return;
                }
                if self.s.arrangement.with_untracked(|a| a.channel_owner(c).is_some_and(|o| o != lane)) {
                    return self.set_status(format!("{} is taken: unlock its track first.", tracks::channel_name(c)));
                }
                if self.s.arrangement.with_untracked(|a| a.lock_resets(lane)) {
                    let text = lock_reset_text(&name, db, pan);
                    if self.ask(cx, &text, "Reset and lock", None) != Some(true) {
                        return;
                    }
                }
                // A locked track plays one note at a time: chords and overlaps
                // are a profile conflict (§5.5).
                if !self.check_mono_for_lock(cx, lane) {
                    return;
                }
                self.s.arrangement.update(|a| {
                    if lock.is_some() {
                        a.unlock_track(lane);
                    }
                    a.lock_track(lane, c);
                });
                self.commit();
                self.set_status(format!("\"{name}\" is locked to the {}.", tracks::channel_name(c)));
            }
        }
        if crate::dev::enabled() {
            let l = self.s.arrangement.with_untracked(|a| a.lanes.get(lane).map(|l| (l.lock, l.vol_db, l.pan)));
            eprintln!("dev: lock track {lane} -> {l:?}");
        }
    }
}

// ───────────────────────────── Profile conflicts ─────────────────────────────

/// One note that breaks a locked track's hardware rules: (track, clip, note).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Conflict {
    pub kind: u8,
    pub track: usize,
    pub clip: u32,
    pub note: u32,
}

fn kind_code(k: WarnKind) -> u8 {
    WarnKind::ALL.iter().position(|x| *x == k).unwrap_or(0) as u8
}

fn osc_of(chan: usize) -> Option<chip::pitch::Osc> {
    use chip::pitch::Osc;
    match chan {
        0 | 1 => Some(Osc::Pulse),
        2 => Some(Osc::Triangle),
        5 | 6 => Some(Osc::Vrc6Pulse),
        7 => Some(Osc::Saw),
        _ => None,
    }
}

/// Whether console channel `chan` can play `pitch` (melodic channels: a
/// period exists; noise and DPCM rows always map).
pub fn playable(chan: usize, pitch: u8) -> bool {
    osc_of(chan).is_none_or(|o| chip::pitch::period_for(o, pitch).is_some())
}

/// The nearest playable pitch by whole octaves (the Fit for a range conflict).
pub fn fold_into_range(chan: usize, pitch: u8) -> u8 {
    let mut p = pitch as i32;
    for _ in 0..12 {
        if playable(chan, p as u8) {
            break;
        }
        p += if p < 60 { 12 } else { -12 };
        p = p.clamp(0, 127);
    }
    p as u8
}

/// Every note on a locked track (in the profile) that the hardware can't
/// play as written: a chord or overlap (one note at a time), a pitch out of
/// the channel's range, a DPCM row with no sample.
pub fn conflicts(a: &crate::state::Arrangement, chip: crate::state::SoundChip) -> Vec<Conflict> {
    let mut out = Vec::new();
    for c in &a.clips {
        let Some(t) = a.lanes.get(c.lane) else { continue };
        let Some(chan) = t.lock else { continue };
        if !chip.has_channel(chan) {
            continue;
        }
        let notes: Vec<&crate::notes::Note> = c.audible_notes().collect();
        for (i, n) in notes.iter().enumerate() {
            // A note that starts before the previous one ends, or at the same time.
            if i > 0 && notes[..i].iter().any(|m| m.end().min(c.len_ticks) > n.start) {
                out.push(Conflict { kind: kind_code(WarnKind::Chord), track: c.lane, clip: c.id, note: n.id });
            }
            if !playable(chan, n.pitch) {
                out.push(Conflict { kind: kind_code(WarnKind::Range), track: c.lane, clip: c.id, note: n.id });
            }
            if chan == chip::driver::LANE_DMC {
                let fixed = t.params[chip::driver::ParamId::Sample as usize];
                let slot = if fixed >= 0 { fixed as usize } else { chip::samples::sample_for_pitch(n.pitch) };
                let present = slot < chip::samples::BUILTIN_COUNT || a.samples.get(slot - chip::samples::BUILTIN_COUNT).is_some_and(|s| s.is_some());
                if !present {
                    out.push(Conflict { kind: kind_code(WarnKind::Dpcm), track: c.lane, clip: c.id, note: n.id });
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

impl AppState {
    /// The main window's handle (the warning dialog's parent).
    fn warn_parent(&self) -> Option<isize> {
        self.hwnd
    }

    /// Asks a profile warning, or gives the remembered answer; a ticked
    /// "Don't ask again" remembers this one.
    pub(crate) fn warn(&mut self, kind: WarnKind, q: &crate::warn::Ask) -> Choice {
        if let Some(c) = crate::warn::remembered(&self.settings.silenced, kind) {
            if (c == Choice::Unlock && q.unlock.is_some()) || (c == Choice::Fit && q.fit.is_some()) {
                return c;
            }
        }
        let (c, dont) = crate::warn::show(q, self.warn_parent());
        self.last_activity = std::time::Instant::now();
        if dont {
            self.settings.silenced.retain(|s| !s.starts_with(&format!("{}=", kind.id())));
            self.settings.silenced.push(crate::warn::silenced_entry(kind, c));
            self.save_settings();
        }
        c
    }

    /// Before an edit is committed: new conflicts on locked tracks ask first
    /// (§5.5). Returns false if the edit was cancelled (the document went
    /// back to the last undo step). Conflicts that were already there (an old
    /// file) never ask: they show as a status-bar problem.
    pub(crate) fn resolve_conflicts(&mut self) -> bool {
        let chip = self.s.chip.get_untracked();
        let now = self.s.arrangement.with_untracked(|a| conflicts(a, chip));
        let new: Vec<Conflict> = now.iter().copied().filter(|c| !self.conflicts_seen.contains(c)).collect();
        if new.is_empty() {
            self.conflicts_seen = now;
            return true;
        }
        for kind in [WarnKind::Chord, WarnKind::Range, WarnKind::Dpcm] {
            let these: Vec<Conflict> = new.iter().copied().filter(|c| c.kind == kind_code(kind)).collect();
            if these.is_empty() {
                continue;
            }
            let mut tracks: Vec<usize> = these.iter().map(|c| c.track).collect();
            tracks.dedup();
            let names: Vec<String> = self.s.arrangement.with_untracked(|a| {
                tracks.iter().filter_map(|&t| a.lanes.get(t).map(|l| format!("\"{}\" ({})", l.name, tracks::channel_name(l.lock.unwrap_or(0))))).collect()
            });
            let who = names.join(", ");
            let (title, text, unlock, fit) = match kind {
                WarnKind::Chord => (
                    "A locked track plays one note at a time",
                    format!("{who}: this edit makes a chord or overlapping notes, which the real chip can't play.\n\nUnlock the track to keep the chord (a studio track, unlimited voices), or fit it to the hardware: each note is cut where the next one starts (the last note wins)."),
                    Some("Unlock track"),
                    Some("Fit to hardware"),
                ),
                WarnKind::Range => (
                    "A note is outside the channel's range",
                    format!("{who}: this edit puts {} note(s) where the channel has no pitch (they would be silent).\n\nFit to hardware moves them by octaves into the channel's range.", these.len()),
                    None,
                    Some("Fit to hardware"),
                ),
                _ => (
                    "A DPCM row has no sample",
                    format!("{who}: {} note(s) are on DPCM rows with no sample loaded, so the chip plays nothing there.\n\nFit to hardware removes them; or import a sample into that slot first.", these.len()),
                    None,
                    Some("Fit to hardware"),
                ),
            };
            let q = crate::warn::Ask { title, text: &text, unlock, fit };
            match self.warn(kind, &q) {
                Choice::Unlock => {
                    self.s.arrangement.update(|a| tracks.iter().for_each(|&t| a.unlock_track(t)));
                }
                Choice::Fit => self.fit_conflicts(kind, &these),
                Choice::Cancel => {
                    let back = self.history.current().clone();
                    self.apply_doc(&back);
                    self.conflicts_seen = self.s.arrangement.with_untracked(|a| conflicts(a, chip));
                    return false;
                }
            }
        }
        self.conflicts_seen = self.s.arrangement.with_untracked(|a| conflicts(a, chip));
        true
    }

    /// Fit to hardware for `these` (all of one kind).
    fn fit_conflicts(&mut self, kind: WarnKind, these: &[Conflict]) {
        let mut clips: Vec<u32> = these.iter().map(|c| c.clip).collect();
        clips.dedup();
        self.s.arrangement.update(|a| {
            for id in &clips {
                let chan = a.clip(*id).and_then(|c| a.lanes.get(c.lane)).map_or(0, |l| l.chan());
                let Some(c) = a.clip_mut(*id) else { continue };
                let mut next = c.next_note_id;
                match kind {
                    // The tracker rule: each note is cut at the next one's start.
                    WarnKind::Chord => crate::notes::resolve_mono(&mut c.notes, &[], &mut next),
                    WarnKind::Range => {
                        for n in c.notes.iter_mut() {
                            if these.iter().any(|x| x.clip == *id && x.note == n.id) {
                                n.pitch = fold_into_range(chan, n.pitch);
                            }
                        }
                        crate::notes::resolve_mono(&mut c.notes, &[], &mut next);
                    }
                    _ => c.notes.retain(|n| !these.iter().any(|x| x.clip == *id && x.note == n.id)),
                }
                c.next_note_id = next;
            }
        });
    }

    /// Locking a studio track whose clips have chords: fit them first, or
    /// don't lock (§5.5; Unlock doesn't apply: it isn't locked yet).
    pub(crate) fn check_mono_for_lock(&mut self, _cx: &mut EventContext, lane: usize) -> bool {
        let chords: Vec<u32> = self.s.arrangement.with_untracked(|a| a.clips.iter().filter(|c| c.lane == lane && !crate::notes::is_mono(&c.notes)).map(|c| c.id).collect());
        if chords.is_empty() {
            return true;
        }
        let name = self.s.arrangement.with_untracked(|a| a.lanes[lane].name.clone());
        let text = format!("\"{name}\" has chords or overlapping notes in {} clip(s). A locked track plays one note at a time.\n\nFit to hardware cuts each note where the next one starts (the last note wins), then locks the track.", chords.len());
        let q = crate::warn::Ask { title: "Lock a track with chords?", text: &text, unlock: None, fit: Some("Fit and lock") };
        match self.warn(WarnKind::Chord, &q) {
            Choice::Fit => {
                self.s.arrangement.update(|a| {
                    for id in &chords {
                        if let Some(c) = a.clip_mut(*id) {
                            let mut next = c.next_note_id;
                            crate::notes::resolve_mono(&mut c.notes, &[], &mut next);
                            c.next_note_id = next;
                        }
                    }
                });
                true
            }
            _ => false,
        }
    }

    /// Switching to profile `to` (§5.5): the tracks locked to channels it
    /// lacks are listed; unlock them (studio tracks, still playing), keep
    /// them locked (dimmed and silent until you switch back), or cancel.
    pub(crate) fn profile_switch_ok(&mut self, to: crate::state::SoundChip) -> bool {
        let stranded: Vec<(usize, String)> = self.s.arrangement.with_untracked(|a| {
            a.lanes.iter().enumerate().filter(|(_, l)| l.lock.is_some_and(|c| !to.has_channel(c))).map(|(i, l)| (i, format!("\"{}\" ({})", l.name, tracks::channel_name(l.lock.unwrap_or(0))))).collect()
        });
        if stranded.is_empty() {
            return true;
        }
        let list = stranded.iter().map(|x| x.1.clone()).collect::<Vec<_>>().join("\n");
        let text = format!("{} doesn't have the channels these locked tracks use:\n\n{list}\n\nUnlock them to keep them playing as studio tracks, or switch anyway: they stay locked, dimmed and silent until you switch back.", to.label());
        let title = format!("Switch the hardware profile to {}?", to.label());
        let q = crate::warn::Ask { title: &title, text: &text, unlock: Some("Unlock tracks"), fit: Some("Switch anyway") };
        match self.warn(WarnKind::Profile, &q) {
            Choice::Unlock => {
                self.s.arrangement.update(|a| stranded.iter().for_each(|(i, _)| a.unlock_track(*i)));
                true
            }
            Choice::Fit => true,
            Choice::Cancel => false,
        }
    }
}
