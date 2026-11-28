//! Presets + Browser + device panel (PRESETS_PROMPT.md §8.4): the bank and
//! the preset model, offline.
//!
//! - PB-B1 every preset loads, renders, doesn't clip, has a provenance entry,
//!   and (authentic) uses no studio-only control;
//! - PB-B2 the Voice switch is one undo step and keeps the track;
//! - PB-B3 a preset remembers its voice (studio), a locked track keeps its
//!   channel, and loading an authentic preset sets nothing studio-only;
//! - PB-B4 ◀ ▶: a run of steps folds into one undo step (`History::amend`);
//! - PB-B5 the bank's names follow VISION.md (no game, console or brand name).

use std::sync::Arc;

use chip::voice::VoiceKind;
use ui::bank;
use ui::file::Project;
use ui::history::{Doc, History};
use ui::state::{Clip, SoundChip, Unison, TICKS_PER_BAR};
use ui::theme::tokens::Rgba;
use ui::user_presets::{view, PresetView};

/// A one-bar project playing preset `i`'s phrase on one track (an authentic
/// preset on a track locked to its channel under its profile).
fn project_for(i: usize) -> Project {
    let p = bank::get(i).unwrap();
    let mut pr = Project::new_empty();
    pr.arrangement.lanes.clear();
    pr.arrangement.instruments.lanes.clear();
    let lane = pr.arrangement.insert_track(0, p.voice, Rgba::hex(0x4FA8E0));
    if let Some(prof) = p.profile {
        pr.chip = prof;
        pr.filter = ui::tracks::default_filter(prof);
        pr.arrangement.lock_track(lane, ui::tracks::channels_of(p.voice)[0]);
    }
    ui::preset_ui::apply_view(&mut pr.arrangement, lane, &view(i, &[]).unwrap());
    let ph = bank::phrase_for(&p.phrase);
    // One bar of the phrase (the listening pack plays all of it).
    let len = TICKS_PER_BAR;
    let notes = bank::phrase_notes(ph, p.octave).into_iter().filter(|n| n.start < len).collect();
    pr.arrangement.clips.push(Arc::new(Clip::new(1, lane, 0, len, ph.name.into(), notes)));
    pr.loop_end_ticks = len;
    pr
}

#[test]
fn pb_b1_every_preset_loads_renders_without_clipping_and_has_provenance() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let prov = std::fs::read_to_string(root.join("presets/PROVENANCE.md")).expect("presets/PROVENANCE.md");
    let mut silent = Vec::new();
    for (i, p) in bank::all().iter().enumerate() {
        let pr = project_for(i);
        // It loaded: the track plays the preset's instrument and voice.
        let lane = &pr.arrangement.lanes[0];
        assert!(p.fits_voice(lane.voice), "{}: voice {:?}", p.name, lane.voice);
        assert_eq!(pr.arrangement.instruments.lane(0).unwrap().name, p.inst.name, "{}", p.name);
        assert_eq!(lane.lock.is_some(), p.authentic(), "{}", p.name);
        let (x, _) = ui::export::render_channels(&pr);
        assert!(x.iter().all(|v| v.is_finite()), "{}", p.name);
        let peak = x.iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak < 1.0, "{} clips (peak {peak})", p.name);
        if peak < 1e-3 {
            silent.push(p.name.clone());
        }
        // Its own provenance row, credited and "designed from scratch".
        let row = prov.lines().find(|l| l.split('|').nth(2).is_some_and(|c| c.trim() == p.name)).unwrap_or_else(|| panic!("{}: no provenance row", p.name));
        assert!(row.contains(bank::AUTHOR) && row.contains("designed from scratch"), "{}: {row}", p.name);
        // Authentic: only what the hardware has.
        if p.authentic() {
            assert_eq!(p.unison, Unison::default(), "{}", p.name);
            assert_eq!(lane.unison, Unison::default(), "{}", p.name);
        }
    }
    assert!(silent.is_empty(), "silent: {silent:?}");
}

/// The track's Voice as a document: one undo step, the old voice back.
#[test]
fn pb_b2_voice_switch_is_one_undo_step() {
    let mut p = Project::new_empty();
    p.arrangement.lanes.clear();
    p.arrangement.instruments.lanes.clear();
    let lane = p.arrangement.insert_track(0, VoiceKind::Pulse, Rgba::hex(0x4FA8E0));
    let a = &mut p.arrangement;
    let mut h = History::new(Doc::of(a, 150.0, a.console_params()));
    let before = a.lanes[lane].clone();
    assert!(a.set_voice(lane, VoiceKind::Saw));
    assert_eq!(a.lanes[lane].voice, VoiceKind::Saw);
    assert_eq!(a.lanes[lane].params, engine::Params::default().values[VoiceKind::Saw.lane()], "the saw's own controls");
    assert_eq!((a.lanes[lane].id, &a.lanes[lane].name), (before.id, &before.name), "the same track");
    assert!(h.commit(Doc::of(a, 150.0, a.console_params())));
    assert_eq!(h.steps(), (1, 0));
    let back = h.undo(Doc::of(a, 150.0, a.console_params())).unwrap();
    back.apply_to(a);
    assert_eq!(a.lanes[lane].voice, VoiceKind::Pulse);
    assert_eq!(a.lanes[lane].params, before.params);
    // A locked track's voice is its channel's.
    a.lock_track(lane, 0);
    assert!(!a.set_voice(lane, VoiceKind::Noise));
    assert_eq!(a.lanes[lane].voice, VoiceKind::Pulse);
}

fn studio_track(v: VoiceKind) -> Project {
    let mut p = Project::new_empty();
    p.arrangement.lanes.clear();
    p.arrangement.instruments.lanes.clear();
    p.arrangement.insert_track(0, v, Rgba::hex(0x4FA8E0));
    p
}

fn pv(name: &str) -> PresetView {
    view(bank::index_of(name).unwrap_or_else(|| panic!("{name}")), &[]).unwrap()
}

#[test]
fn pb_b3_a_preset_remembers_its_voice_and_a_lock_keeps_its_channel() {
    // A studio Pulse track takes a saw lead: it becomes a saw track with the
    // preset's Unison / Detune / Drift.
    let mut p = studio_track(VoiceKind::Pulse);
    ui::preset_ui::apply_view(&mut p.arrangement, 0, &pv("Detuned Saw Lead"));
    let l = &p.arrangement.lanes[0];
    assert_eq!(l.voice, VoiceKind::Saw);
    assert_eq!(l.unison, Unison { voices: 3, detune: 14, drift: 2 });
    // A classic preset that suits the track's voice keeps it (Flute: pulse,
    // VRC6, saw, triangle) and sets no unison.
    let mut q = studio_track(VoiceKind::Triangle);
    ui::preset_ui::apply_view(&mut q.arrangement, 0, &pv("Flute"));
    assert_eq!(q.arrangement.lanes[0].voice, VoiceKind::Triangle);
    // A locked track keeps its channel and ignores studio-only controls.
    let mut r = studio_track(VoiceKind::Pulse);
    r.chip = SoundChip::Base2A03;
    r.arrangement.lock_track(0, 1);
    let v = pv("Supersquare");
    assert!(v.studio_only() && v.fits(1));
    ui::preset_ui::apply_view(&mut r.arrangement, 0, &v);
    assert_eq!((r.arrangement.lanes[0].lock, r.arrangement.lanes[0].unison), (Some(1), Unison::default()));
    // Authentic presets: the Hardware folder's, locking under their profile.
    let hw = pv("Duty Lead");
    assert_eq!(hw.profile, Some(SoundChip::Base2A03));
    assert!(!hw.studio_only());
    assert_eq!(pv("Saw Brass").profile, Some(SoundChip::WithVrc6));
}

/// ◀ ▶ (§6.2): the app commits the first step and amends the others while
/// nothing else was committed: a run of steps is one undo step, and undo
/// goes back to the preset before the run.
#[test]
fn pb_b4_stepping_coalesces_into_one_undo_step() {
    let mut p = studio_track(VoiceKind::Pulse);
    let start = bank::index_of("Bright Square Lead").unwrap();
    ui::preset_ui::apply_view(&mut p.arrangement, 0, &view(start, &[]).unwrap());
    let a = &mut p.arrangement;
    let mut h = History::new(Doc::of(a, 150.0, a.console_params()));
    let mut cur = start;
    for k in 0..5 {
        cur = bank::step(cur, 1).unwrap();
        ui::preset_ui::apply_view(a, 0, &view(cur, &[]).unwrap());
        let d = Doc::of(a, 150.0, a.console_params());
        if k == 0 {
            h.commit(d);
        } else {
            h.amend(d);
        }
    }
    assert_eq!(h.steps(), (1, 0), "five steps, one undo step");
    assert_ne!(cur, start);
    let back = h.undo(Doc::of(a, 150.0, a.console_params())).unwrap();
    back.apply_to(a);
    assert_eq!(a.instruments.lane(0).unwrap().name, "Bright Square Lead");
    // Stepping stays in the folder (it wraps).
    let fold = bank::siblings(start);
    assert!(fold.contains(&cur));
}

#[test]
fn pb_b5_names_are_descriptive() {
    for p in bank::all() {
        let n = p.name.to_lowercase();
        for bad in ["nintendo", "nes ", "famicom", "konami", "capcom", "sega", "mario", "zelda", "metroid", "castlevania", "mega man", "ableton", "tmnt"] {
            assert!(!n.contains(bad), "{}", p.name);
        }
        assert!(!p.blurb.to_lowercase().contains("nintendo"));
    }
}
