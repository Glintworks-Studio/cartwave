//! Batch 4b (BATCH4B_PROMPT.md): stereo, unison and gain staging through the
//! whole offline path (project → mixdown → export).

use ui::{export, file, game_export, render, tracks};

fn v7(name: &str) -> file::Project {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/songs/v7");
    file::load(&dir.join(format!("{name}.chipdaw"))).unwrap()
}

/// The showcase on studio tracks with the pans spread (the stereo golden
/// project, §4.3).
fn panned_showcase() -> file::Project {
    let mut p = v7("v7_studio_showcase");
    for (l, pan) in p.arrangement.lanes.iter_mut().zip([-30i8, 30, 0, 15, -10, -45, 45, 5]) {
        l.pan = pan;
    }
    p
}

/// The stereo golden hash (§4.3): a panned render is deterministic (pure
/// arithmetic: the pan law and drift use `chip::math`, no platform libm), so
/// this value is the same on every machine. Recorded by this build;
/// re-baselined in Batch 5 A3 (studio voices made bipolar, was
/// 0x3e54_3873_23b0_f321: docs/batch5/a3-rebaseline.md).
const STEREO_GOLDEN: u64 = 0x3a95_c377_6a9b_c7d3;

#[test]
fn b4b_stereo_golden_render() {
    let p = panned_showcase();
    assert_eq!(tracks::channels(&p.arrangement), 2);
    let (x, ch) = export::render_channels(&p);
    assert_eq!(ch, 2);
    let h = render::hash(&x);
    // The same render twice is the same (no state leaks between renders).
    assert_eq!(render::hash(&export::render_channels(&p).0), h);
    assert_eq!(h, STEREO_GOLDEN, "stereo golden: {h:#018x}");
}

/// §4.2: exports signal stereo by their channel count. A project with no
/// panned studio track writes mono WAVs (byte-identical to the mono path);
/// with a pan, the mix and every stem are stereo, and the stems still add up
/// to the mix (studio tracks mix linearly).
#[test]
fn b4b_exports_are_mono_until_a_pan_and_stems_follow_the_mix() {
    let mono = v7("v7_studio_demo");
    let o = game_export::Options { range: game_export::Range::Song, ..Default::default() };
    let (x, l) = game_export::render_heard(&mono, 48_000, &tracks::audible(&mono.arrangement, &mono.mute, &mono.solo), o.range, 2, None).unwrap();
    assert_eq!(l.channels, 1);
    assert_eq!(x.len(), l.total);
    let wav = game_export::wav_bytes_ch(&x, 48_000, 16, 1, None);
    assert_eq!(wav, game_export::wav_bytes(&x, 48_000, 16, None), "mono files are the old ones");

    let mut p = v7("v7_studio_demo");
    p.arrangement.lanes[0].pan = -40;
    p.arrangement.lanes[1].pan = 25;
    let n = p.arrangement.lanes.len();
    let (mix, lm) = game_export::render_heard(&p, 48_000, &vec![true; n], o.range, 2, None).unwrap();
    assert_eq!((lm.channels, mix.len()), (2, 2 * lm.total));
    let wav = game_export::wav_bytes_ch(&mix, 48_000, 16, 2, None);
    assert_eq!(u16::from_le_bytes([wav[22], wav[23]]), 2, "fmt: two channels");
    let mut stems = Vec::new();
    for i in 0..n {
        let heard: Vec<bool> = (0..n).map(|j| j == i).collect();
        let (s, ls) = game_export::render_heard(&p, 48_000, &heard, o.range, 2, None).unwrap();
        assert_eq!(ls.channels, 2, "a centred track's stem is stereo too");
        stems.push(s);
    }
    let refs: Vec<&[f32]> = stems.iter().map(|s| s.as_slice()).collect();
    let (rms_db, _) = game_export::sum_error(&mix, &refs);
    assert!(rms_db < -60.0, "studio stems add up to the mix: {rms_db:.1} dB");
}

/// §5: unison changes the sound, and a project's unison is part of the
/// render (deterministic).
#[test]
fn b4b_unison_renders_deterministically() {
    let mut p = v7("v7_studio_demo");
    let base = render::hash(&export::render(&p));
    p.arrangement.lanes[0].unison = ui::state::Unison { voices: 4, detune: 15, drift: 3 };
    let a = render::hash(&export::render(&p));
    assert_ne!(a, base);
    assert_eq!(render::hash(&export::render(&p)), a);
}
