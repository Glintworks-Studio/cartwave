//! Batch 5: automation at the project level (the AU tests that need a whole
//! song: the engine's per-frame register writes from a project's envelope,
//! the game export with automation, its note for studio lanes, the seamless
//! loop with automation). The editing behaviours are unit tests in
//! `src/automation.rs` (AU-01 … AU-04, AU-06, AU-08) and the app scenarios
//! in `docs/batch5/tools/at_b5.ps1`.

use ui::automation::{AutoOwner, Envelope, Pt};
use ui::game_export::{self as gx, Options, Range};
use ui::{export, file, render};

fn scale_test() -> file::Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/scale_test.chipdaw");
    let mut p = file::load(&path).unwrap();
    p.loop_start_ticks = 0;
    p
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("cartwave-auto-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

/// AU-07 / AU-09 (register writes): a locked Pulse 1 with a Volume envelope
/// and a Duty envelope. Driving the console's driver from the project's own
/// snapshot, frame by frame, the control register written for the channel
/// carries exactly the quantised envelope values (volume nibble: note x
/// channel volume, the channel volume being the envelope's step; duty: one of
/// the 2A03's four).
#[test]
fn au07_locked_lanes_play_the_quantised_envelope_frame_by_frame() {
    let mut p = scale_test();
    let lane = p.arrangement.lanes.iter().position(|l| l.lock == Some(0)).expect("a Pulse 1 track");
    let id = p.arrangement.lanes[lane].id;
    let vol = Envelope::new(vec![Pt::new(0, 15.0), Pt::new(7680, 1.0)]);
    let duty = Envelope::new(vec![Pt::new(0, 0.0), Pt { t: 7680, v: 3.0, c: 0.5 }]);
    *p.arrangement.automation.edit(&(AutoOwner::Track(id), "chip.volume".into())) = vol.clone();
    *p.arrangement.automation.edit(&(AutoOwner::Track(id), "chip.duty".into())) = duty.clone();
    let snap = p.arrangement.song_snapshot(1);
    assert_eq!(snap.auto[0].len(), 2, "both envelopes on channel 0");
    let song = snap.view();
    let mut d = chip::driver::Driver::new(chip::samples::build(&mut [0u8; chip::dmc::ROM_SIZE]));
    let mut w = chip::driver::Writes::default();
    let tick = 960.0 * p.bpm as f64 / 60.0 / chip::tables::FRAME_HZ_NTSC;
    let (mut pos, mut prev, mut checked) = (0.0, None, 0);
    while pos < 7600.0 {
        w.clear();
        d.tick(&song, prev, pos, true, &mut w);
        let want_v = chip::auto::quantise(vol.value_at(pos).unwrap(), 0, 15);
        let want_d = chip::auto::quantise(duty.value_at(pos).unwrap(), 0, 3);
        assert_eq!(d.effective(0, chip::driver::ParamId::Volume), want_v, "volume at {pos}");
        assert_eq!(d.effective(0, chip::driver::ParamId::Duty), want_d, "duty at {pos}");
        if let Some(&(_, v)) = w.as_slice().iter().rev().find(|(a, _)| *a == 0x4000) {
            // A note sounding: its register carries them.
            if v & 15 != 0 {
                assert_eq!((v >> 6) as i32, want_d, "duty bits at {pos}");
                checked += 1;
            }
        }
        prev = Some(pos);
        pos += tick;
    }
    assert!(checked > 10, "registers checked on {checked} frames");
}

/// AU-09: the game export with automation. A locked track's envelope is in
/// the audio (it differs from the same song without it) and needs no note;
/// a studio track's envelope is rendered too, and the report names the
/// track and lane. The loop stays seamless with automation in it.
#[test]
fn au09_game_export_with_automation() {
    let mut p = scale_test();
    let plain = render::hash(&export::render_channels(&p).0);
    let id0 = p.arrangement.lanes[0].id;
    *p.arrangement.automation.edit(&(AutoOwner::Track(id0), "chip.volume".into())) = Envelope::new(vec![Pt::new(0, 15.0), Pt::new(3840, 2.0)]);
    assert_ne!(render::hash(&export::render_channels(&p).0), plain, "the locked envelope is heard");
    assert!(gx::studio_automation_note(&p).is_none(), "locked automation plays on hardware: no note");
    // A studio track with a fader envelope.
    let t = p.arrangement.insert_track(p.arrangement.lanes.len(), chip::voice::VoiceKind::Pulse, ui::theme::tokens::Rgba::hex(0x4FA8E0));
    p.arrangement.lanes[t].name = "Lead".into();
    let tid = p.arrangement.lanes[t].id;
    *p.arrangement.automation.edit(&(AutoOwner::Track(tid), "track.gain".into())) = Envelope::new(vec![Pt::new(0, 0.0), Pt::new(3840, -24.0)]);
    let note = gx::studio_automation_note(&p).expect("a note for the studio lane");
    assert!(note.contains("Lead (Volume)"), "{note}");
    // Exported as a seamless loop: intro + loop x 3 = a continuous render.
    let o = Options { range: Range::Loop, ..Options::default() };
    let dir = tmp("loop");
    let rep = gx::export(&p, &o, &dir, "song", &gx::Control::default()).unwrap();
    assert!(rep.notes.iter().any(|n| n.contains("Lead (Volume)")), "{:?}", rep.notes);
    let (a, b) = rep.layout.unwrap().loop_.unwrap();
    let read = |f: &str| gx::read_wav(&std::fs::read(dir.join(f)).unwrap()).unwrap().0;
    let (intro, lp) = (read("song_intro.wav"), read("song_loop.wav"));
    assert_eq!((intro.len(), lp.len()), (a, b - a));
    let (cont, _) = gx::render(&p, 48_000, 0, Range::Loop, 4, None).unwrap();
    let cont: Vec<i32> = cont.iter().map(|&x| gx::quantize(x, 16)).collect();
    let mut joined = intro.clone();
    for _ in 0..3 {
        joined.extend_from_slice(&lp);
    }
    assert_eq!(joined.iter().zip(&cont).position(|(x, y)| x != y), None, "seamless with automation");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Deterministic: the same project renders bit-identically twice, with
/// automation on locked tracks, studio tracks and Main.
#[test]
fn automated_renders_are_deterministic() {
    let mut p = scale_test();
    let id0 = p.arrangement.lanes[0].id;
    *p.arrangement.automation.edit(&(AutoOwner::Track(id0), "macro.pitch".into())) = Envelope::new(vec![Pt::new(0, -50.0), Pt::new(7680, 50.0)]);
    *p.arrangement.automation.edit(&(AutoOwner::Main, "main.gain".into())) = Envelope::new(vec![Pt::new(0, 0.0), Pt { t: 7680, v: -12.0, c: -0.5 }]);
    let a = export::render_channels(&p).0;
    let b = export::render_channels(&p).0;
    assert_eq!(render::hash(&a), render::hash(&b));
}
