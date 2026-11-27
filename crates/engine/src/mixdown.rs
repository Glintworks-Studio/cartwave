//! Everything meets at the Main track (BATCH4A_PROMPT.md §5.6, §7): the
//! console (locked tracks through the full chip emulator and its non-linear
//! mixer), the studio bus (every studio track's voices), and the master gain.
//!
//! The same `Mixdown` runs in the engine (`core.rs`), in offline renders and
//! in every export, so what you hear is what you export.
//!
//! **Bit-identity.** With no studio tracks and the master at 0 dB, `render`
//! is `Player::render` exactly: no hook, no studio bus, no multiply. So every
//! project from before the model change renders as it did (the golden render
//! and the migration hashes).
//!
//! **Stereo (Batch 4b §4).** Studio tracks pan (`studio::pan_gains`); the
//! console and locked tracks stay centred, as the hardware is mono. The bus
//! keeps three step buffers (centre, left only, right only): until a studio
//! track is panned off centre only the centre has steps and the render is the
//! mono one, bit for bit. `render_stereo` reports whether the sides are in use.
//!
//! **Gain staging (Batch 4b §5, RUNLOG B4B-D*).** A studio voice plays at the
//! console's level (one note: +0.000 dB). Unison voices are trimmed by
//! `1/√n` (`studio::unison_trim`). The studio bus then has a safety limiter:
//! a linked peak limiter at −1 dBFS with an instant attack and a 150 ms
//! release, so chords, unison and many tracks can't drive the output past
//! full scale. It does nothing (gain exactly 1.0, no multiply) while the bus
//! stays under its ceiling, which a single note always does.

use crate::song::SongSnapshot;
use crate::studio::{Rack, Steps};
use chip::blep::MAX_FRAMES;
use chip::dmc::ROM_SIZE;
use chip::driver::Song;
use chip::filter::{FilterMode, OutputFilter};
use chip::player::{Player, TickHook};
use chip::samples::{SampleRef, USER_SLOTS};
use chip::tables::CPU_HZ_NTSC;

/// The safety limiter's ceiling: −1 dBFS.
pub const LIMIT_CEIL: f32 = 0.891_250_9;
/// Its release time constant (seconds).
const LIMIT_RELEASE_S: f64 = 0.150;

/// The studio tracks' shared step buffers (centre, left only, right only),
/// their output filters and the safety limiter.
#[derive(Clone)]
pub struct Bus {
    steps: Steps,
    filter: OutputFilter,
    filter_l: OutputFilter,
    filter_r: OutputFilter,
    scratch: Box<[f32; MAX_FRAMES]>,
    /// The sides of the last chunk (left only, right only), after the limiter.
    side_l: Box<[f32; MAX_FRAMES]>,
    side_r: Box<[f32; MAX_FRAMES]>,
    /// The limiter's gain now (exactly 1.0 when it isn't working) and its
    /// per-sample release step.
    limit: f32,
    release: f32,
    /// The bus follows the console's buffer timing (set on first use).
    synced: bool,
}

impl Bus {
    fn new(sample_rate: f64, mode: FilterMode) -> Self {
        Self {
            steps: Steps::new(CPU_HZ_NTSC, sample_rate),
            filter: OutputFilter::new(mode, sample_rate),
            filter_l: OutputFilter::new(mode, sample_rate),
            filter_r: OutputFilter::new(mode, sample_rate),
            scratch: Box::new([0.0; MAX_FRAMES]),
            side_l: Box::new([0.0; MAX_FRAMES]),
            side_r: Box::new([0.0; MAX_FRAMES]),
            limit: 1.0,
            release: (1.0 - (-1.0 / (LIMIT_RELEASE_S * sample_rate)).exp()) as f32,
            synced: false,
        }
    }

    /// The limiter's gain for a sample whose louder channel is `peak`: drops
    /// at once to keep it at the ceiling, recovers over the release; snaps
    /// back to exactly 1.0.
    fn limiter(limit: &mut f32, release: f32, peak: f32) -> f32 {
        let want = if peak > LIMIT_CEIL { LIMIT_CEIL / peak } else { 1.0 };
        let g = if want < *limit { want } else { *limit + (1.0 - *limit) * release };
        *limit = if g > 0.999_99 { 1.0 } else { g };
        limit.min(want)
    }
}

#[derive(Clone)]
pub struct Mixdown {
    pub player: Player,
    pub rack: Rack,
    bus: Bus,
    /// The master gain (linear): the target and where the ramp is.
    master: f32,
    master_now: f32,
    /// The imported DPCM samples' places (for voices added later).
    user: [SampleRef; USER_SLOTS],
    /// The studio bus limiter's smallest gain since the last
    /// `take_limiter` (1.0: it didn't work).
    limiter_min: f32,
}

struct Hook<'a> {
    rack: &'a mut Rack,
    bus: &'a mut Bus,
    song: &'a Song<'a>,
    snap: &'a SongSnapshot,
    /// The limiter's smallest gain this chunk (for the meters and tests).
    min_gain: f32,
}

impl TickHook for Hook<'_> {
    fn tick(&mut self, rom: &[u8; ROM_SIZE], cycle: u64, prev: Option<f64>, pos: f64, playing: bool) {
        self.rack.tick(self.song, &self.snap.studio, cycle, prev, pos, playing, rom, &mut self.bus.steps);
    }

    fn chunk(&mut self, rom: &[u8; ROM_SIZE], end: u64, out: &mut [f32]) {
        let bus = &mut *self.bus;
        self.rack.chunk(end, rom, &mut bus.steps);
        let n = out.len();
        let buf = &mut bus.scratch[..n];
        bus.steps.c.read(buf, end);
        if !bus.steps.side {
            // Mono (no track panned yet): the centre only, as before.
            for (o, v) in out.iter_mut().zip(buf.iter()) {
                let c = bus.filter.process(*v);
                let g = if c.abs() > LIMIT_CEIL || bus.limit != 1.0 { Bus::limiter(&mut bus.limit, bus.release, c.abs()) } else { 1.0 };
                self.min_gain = self.min_gain.min(g);
                *o += if g == 1.0 { c } else { c * g };
            }
            return;
        }
        let (sl, sr) = (&mut bus.side_l[..n], &mut bus.side_r[..n]);
        bus.steps.l.read(sl, end);
        bus.steps.r.read(sr, end);
        for i in 0..n {
            let c = bus.filter.process(buf[i]);
            let (l, r) = (bus.filter_l.process(sl[i]), bus.filter_r.process(sr[i]));
            let peak = (c + l).abs().max((c + r).abs());
            let g = if peak > LIMIT_CEIL || bus.limit != 1.0 { Bus::limiter(&mut bus.limit, bus.release, peak) } else { 1.0 };
            self.min_gain = self.min_gain.min(g);
            out[i] += c * g;
            sl[i] = l * g;
            sr[i] = r * g;
        }
    }
}

impl Mixdown {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            player: Player::new(sample_rate),
            rack: Rack::default(),
            bus: Bus::new(sample_rate, FilterMode::default()),
            master: 1.0,
            master_now: 1.0,
            user: [SampleRef::default(); USER_SLOTS],
            limiter_min: 1.0,
        }
    }

    /// The studio bus limiter's smallest gain since the last call (1.0 = it
    /// didn't work), and resets it.
    pub fn take_limiter(&mut self) -> f32 {
        std::mem::replace(&mut self.limiter_min, 1.0)
    }

    /// Whether a studio track has been panned off centre (the output is
    /// stereo from then on).
    pub fn stereo(&self) -> bool {
        self.bus.steps.side
    }

    /// The master gain (linear, the Main track's fader): applied after the
    /// full mix, ramped over the next block.
    pub fn set_master(&mut self, g: f32) {
        self.master = g.max(0.0);
    }

    pub fn master(&self) -> f32 {
        self.master
    }

    pub fn set_filter(&mut self, mode: FilterMode) {
        self.player.synth.set_filter(mode);
        if mode != self.bus.filter.mode {
            let sr = self.player.synth.sample_rate();
            self.bus.filter = OutputFilter::new(mode, sr);
            self.bus.filter_l = OutputFilter::new(mode, sr);
            self.bus.filter_r = OutputFilter::new(mode, sr);
        }
    }

    /// A new output rate (no stream running): the console and the bus.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.player.synth.set_sample_rate(sample_rate);
        let mode = self.bus.filter.mode;
        self.bus = Bus::new(sample_rate, mode);
    }

    /// Takes a new rack (from the UI), keeping the sounding voices; returns
    /// the old one for the UI thread to free.
    pub fn adopt_rack(&mut self, mut new: Box<Rack>) -> Box<Rack> {
        new.adopt(&mut self.rack, self.player.synth.cycle, &self.user);
        std::mem::swap(&mut self.rack, &mut *new);
        new
    }

    /// Installs imported DPCM samples (already copied into the ROM) for the
    /// console and every voice.
    pub fn set_user_samples(&mut self, refs: [SampleRef; USER_SLOTS]) {
        self.player.driver.set_user_samples(refs);
        self.rack.set_user_samples(&refs);
        self.user = refs;
    }

    /// Renders mono samples: console + studio bus, times the master gain.
    /// (A panned track's side share isn't in it: use `render_stereo` once a
    /// project can be stereo.)
    pub fn render(&mut self, snap: &SongSnapshot, out: &mut [f32]) {
        self.render_parts(snap, out, None);
        self.apply_master(out, None);
    }

    /// Renders both channels: `l` and `r` get the console and the studio
    /// bus, a panned track's sides, times the master gain. Returns whether
    /// the output is stereo (a studio track has been panned): if not, `r` is
    /// a copy of `l`, which is exactly `render`'s output.
    pub fn render_stereo(&mut self, snap: &SongSnapshot, l: &mut [f32], r: &mut [f32]) -> bool {
        debug_assert_eq!(l.len(), r.len());
        self.render_parts(snap, l, Some(r));
        let stereo = self.bus.steps.side;
        if stereo {
            self.apply_master(l, Some(r));
        } else {
            self.apply_master(l, None);
            r.copy_from_slice(l);
        }
        stereo
    }

    /// The mix before the master gain: the centre into `out`; with `right`,
    /// the right channel there and the left side added into `out`.
    fn render_parts(&mut self, snap: &SongSnapshot, out: &mut [f32], mut right: Option<&mut [f32]>) {
        let song = snap.view();
        if self.rack.is_empty() {
            self.bus.synced = false;
            self.player.render(&song, out);
            if let Some(r) = right {
                r.copy_from_slice(out);
            }
            return;
        }
        if !self.bus.synced {
            // Start the bus on the console's sample grid (all three buffers).
            let b = self.player.synth.blep();
            self.bus.steps.c.sync_timing(b);
            self.bus.steps.l.sync_timing(b);
            self.bus.steps.r.sync_timing(b);
            self.bus.synced = true;
        }
        // Pieces of at most one chunk (the player's own chunking), so each
        // piece's sides are the bus's last chunk.
        let mut at = 0;
        while at < out.len() {
            let n = (out.len() - at).min(MAX_FRAMES);
            let piece = &mut out[at..at + n];
            let mut hook = Hook { rack: &mut self.rack, bus: &mut self.bus, song: &song, snap, min_gain: 1.0 };
            self.player.render_with(&song, piece, &mut hook);
            let min_gain = hook.min_gain;
            self.limiter_min = self.limiter_min.min(min_gain);
            if let Some(r) = right.as_deref_mut() {
                let r = &mut r[at..at + n];
                if self.bus.steps.side {
                    for i in 0..n {
                        r[i] = piece[i] + self.bus.side_r[i];
                        piece[i] += self.bus.side_l[i];
                    }
                } else {
                    r.copy_from_slice(piece);
                }
            }
            at += n;
        }
    }

    fn apply_master(&mut self, out: &mut [f32], mut right: Option<&mut [f32]>) {
        if self.master == 1.0 && self.master_now == 1.0 {
            return;
        }
        let (g0, g1) = (self.master_now, self.master);
        if g0 == g1 {
            for v in out.iter_mut() {
                *v *= g1;
            }
            if let Some(r) = right.as_deref_mut() {
                for v in r.iter_mut() {
                    *v *= g1;
                }
            }
        } else {
            // De-zippered: a linear ramp across this block.
            let n = out.len().max(1) as f32;
            for (i, v) in out.iter_mut().enumerate() {
                *v *= g0 + (g1 - g0) * (i + 1) as f32 / n;
            }
            if let Some(r) = right.as_deref_mut() {
                for (i, v) in r.iter_mut().enumerate() {
                    *v *= g0 + (g1 - g0) * (i + 1) as f32 / n;
                }
            }
        }
        self.master_now = g1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::{ClipInput, StudioLane};
    use crate::studio::{max_polyphony, pool_size, StudioTrack, TrackCtl, PARAMS};
    use chip::driver::{Note, Params};
    use chip::instrument::NONE;
    use chip::voice::VoiceKind;
    use std::sync::Arc;

    fn pulse_params() -> [i8; PARAMS] {
        Params::default().values[0]
    }

    fn rack_with(kind: VoiceKind, voices: usize, cap: u16) -> (Box<Rack>, Arc<TrackCtl>) {
        let ctl = Arc::new(TrackCtl::new(1.0, true, cap, pulse_params()));
        let samples = chip::samples::build(&mut [0u8; ROM_SIZE]);
        (Box::new(Rack::new(vec![StudioTrack::new(7, kind, ctl.clone(), voices, samples)])), ctl)
    }

    fn chord(pitches: &[u8], start: u32, len: u32) -> Vec<(u32, u32, u8, Option<u8>)> {
        pitches.iter().map(|&p| (start, len, p, None)).collect()
    }

    fn snap_with(notes: &[(u32, u32, u8, Option<u8>)]) -> SongSnapshot {
        let lane = StudioLane::build(7, NONE, [ClipInput { lane: 0, start: 0, len: 96_000, inst: NONE, notes }]);
        SongSnapshot::default().with_studio(vec![lane])
    }

    fn play(m: &mut Mixdown, snap: &SongSnapshot, secs: f64) -> Vec<f32> {
        m.player.set_bpm(150.0);
        m.player.play();
        let mut out = vec![0f32; (secs * 48_000.0) as usize];
        for c in out.chunks_mut(256) {
            m.render(snap, c);
        }
        out
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len().max(1) as f64).sqrt()
    }

    /// No studio track and the master at 0 dB: exactly the console's render.
    #[test]
    fn no_studio_tracks_is_the_console_bit_for_bit() {
        let notes = [(0, 1920, 60u8, None), (1920, 960, 64, None)];
        let snap = SongSnapshot::build([ClipInput { lane: 0, start: 0, len: 3840, inst: NONE, notes: &notes }], 1);
        let mut m = Mixdown::new(48_000.0);
        let mut p = Player::new(48_000.0);
        p.set_bpm(150.0);
        p.play();
        let a = play(&mut m, &snap, 1.0);
        let mut b = vec![0f32; a.len()];
        for c in b.chunks_mut(256) {
            p.render(&snap.view(), c);
        }
        assert!(a.iter().zip(&b).all(|(x, y)| x.to_bits() == y.to_bits()));
    }

    /// A studio track plays an 8-note chord on 8 voices at once.
    #[test]
    fn a_chord_sounds_on_eight_voices() {
        let notes = chord(&[48, 52, 55, 59, 62, 65, 69, 72], 0, 3840);
        let snap = snap_with(&notes);
        let (rack, ctl) = rack_with(VoiceKind::Pulse, 16, 0);
        let mut m = Mixdown::new(48_000.0);
        m.adopt_rack(rack);
        let out = play(&mut m, &snap, 0.5);
        assert_eq!(ctl.sounding.load(std::sync::atomic::Ordering::Relaxed), 8);
        assert_eq!(ctl.steals.load(std::sync::atomic::Ordering::Relaxed), 0);
        assert!(rms(&out[4800..]) > 0.05);
        assert_eq!(max_polyphony(&snap.studio[0].notes), 8);
    }

    /// An exhausted pool steals the quietest voice: the newest note always
    /// sounds, the pool never grows on the audio thread.
    #[test]
    fn an_exhausted_pool_steals_the_quietest() {
        // Four held notes with falling volumes, then a fifth on a 4-voice pool.
        let notes = vec![(0, 9600, 60u8, Some(15)), (0, 9600, 64, Some(4)), (0, 9600, 67, Some(12)), (0, 9600, 70, Some(9)), (960, 9600, 74, Some(15))];
        let snap = snap_with(&notes);
        let (rack, ctl) = rack_with(VoiceKind::Pulse, 4, 0);
        let mut m = Mixdown::new(48_000.0);
        m.adopt_rack(rack);
        play(&mut m, &snap, 0.6); // past the fifth note (0.4 s)
        assert_eq!(ctl.steals.load(std::sync::atomic::Ordering::Relaxed), 1);
        let held: Vec<u8> = m.rack.tracks[0].slots.iter().filter_map(|s| s.voice.held.map(|h| h.1)).collect();
        assert!(held.contains(&74), "the new note sounds: {held:?}");
        assert!(!held.contains(&64), "the quietest (volume 4) was stolen: {held:?}");
        assert_eq!(m.rack.tracks[0].capacity(), 4);
    }

    /// The Voices setting caps a track below its pool.
    #[test]
    fn the_voices_setting_caps_polyphony() {
        let notes = chord(&[48, 52, 55, 59, 62, 65], 0, 3840);
        let snap = snap_with(&notes);
        let (rack, ctl) = rack_with(VoiceKind::Pulse, 16, 2);
        let mut m = Mixdown::new(48_000.0);
        m.adopt_rack(rack);
        play(&mut m, &snap, 0.3);
        assert_eq!(ctl.sounding.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    /// Growing the pool (a bigger rack from the UI) keeps the sounding voices
    /// and their notes; the old rack comes back to be freed.
    #[test]
    fn pool_growth_keeps_sounding_voices() {
        let notes = chord(&[48, 52, 55, 59], 0, 19_200);
        let snap = snap_with(&notes);
        let (rack, ctl) = rack_with(VoiceKind::Pulse, 4, 0);
        let mut m = Mixdown::new(48_000.0);
        m.adopt_rack(rack);
        play(&mut m, &snap, 0.2);
        let before: Vec<_> = m.rack.tracks[0].slots.iter().map(|s| s.voice.held).collect();
        let samples = chip::samples::build(&mut [0u8; ROM_SIZE]);
        let bigger = Box::new(Rack::new(vec![StudioTrack::new(7, VoiceKind::Pulse, ctl.clone(), pool_size(12, 0), samples)]));
        let old = m.adopt_rack(bigger);
        assert_eq!(old.tracks[0].capacity(), 4);
        assert_eq!(m.rack.tracks[0].capacity(), 24);
        let after: Vec<_> = m.rack.tracks[0].slots.iter().take(4).map(|s| s.voice.held).collect();
        assert_eq!(before, after);
        // And the render goes on without a click: the next block continues.
        let out = play(&mut m, &snap, 0.1);
        assert!(rms(&out) > 0.05);
    }

    /// The master gain scales the whole mix; at 0 dB nothing is touched.
    #[test]
    fn master_gain_scales_the_mix() {
        let notes = [(0, 19_200, 69u8, None)];
        let snap = SongSnapshot::build([ClipInput { lane: 0, start: 0, len: 19_200, inst: NONE, notes: &notes }], 1);
        let mut a = Mixdown::new(48_000.0);
        let mut b = Mixdown::new(48_000.0);
        b.set_master(0.5);
        let x = play(&mut a, &snap, 0.5);
        let y = play(&mut b, &snap, 0.5);
        // After the first block's ramp, exactly half.
        for (p, q) in x[512..].iter().zip(&y[512..]) {
            assert!((p * 0.5 - q).abs() < 1e-7);
        }
    }

    #[test]
    fn pool_sizes_have_headroom() {
        assert_eq!(pool_size(0, 0), 8);
        assert_eq!(pool_size(8, 0), 16);
        assert_eq!(pool_size(8, 4), 4);
        assert!(pool_size(100, 0) >= 150);
        let n = |s, e| Note::new(s, e, 60);
        assert_eq!(max_polyphony(&[n(0, 10), n(5, 20), n(10, 30), n(25, 40)]), 2);
    }
}
