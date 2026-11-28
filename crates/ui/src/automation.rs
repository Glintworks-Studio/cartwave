//! Arrangement automation (Batch 5, VISION.md "Automation", Live 12 manual
//! ch. 26): breakpoint envelopes per track parameter and for the Main gain.
//!
//! **The model.** An envelope belongs to an owner (a track, by its id, or
//! Main) and a parameter of the registry (`params.rs`, the one list: an id
//! such as "chip.duty"). Its points hold values in the parameter's own units
//! (dB, register steps, cents; the fader's −inf is stored as −70 dB) and the
//! curve of the segment that starts at each one. Envelopes are part of the
//! document (undo, files); they are shared (`Arc`) so an undo snapshot costs
//! a pointer each.
//!
//! **What reaches the engine** (`Automation::engine_*`): only parameters the
//! owner has *now* (a locked track has no fader or pan lane: its envelope is
//! kept, unheard, and comes back when the track is unlocked; a parameter its
//! channel doesn't have is skipped), and never an **overridden** one (the
//! control was moved by hand during playback: the manual value plays until
//! Re-Enable Automation).
//!
//! **Editing** (the reference's breakpoint behaviour, manual 26.6–26.7, UX.md
//! §5.6): a click on the line adds a point; a click on a point deletes it
//! (VISION.md, decision 2026-11-25; R108); points drag (never past their
//! neighbours); Alt-drag bends a segment; Draw Mode draws steps at the grid;
//! a time selection's envelope moves, copies, pastes and deletes. Each
//! gesture is one undo step (the app commits once per gesture).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use chip::auto::{AutoEnv, AutoPoint, AutoTarget};
use chip::driver::ParamId;

use crate::params::{self, Owner, Param};
use crate::state::Lane;

/// Who an envelope belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AutoOwner {
    Track(u32),
    Main,
}

/// An envelope's key: its owner and the registry id of its parameter.
pub type AutoKey = (AutoOwner, String);

/// The value stored for the fader at −inf (dB): the engine treats −70 and
/// below as silence.
pub const DB_FLOOR: f32 = -70.0;

/// One breakpoint: tick, value (parameter units), curve of the segment that
/// starts here (−1 … 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub t: u32,
    pub v: f32,
    pub c: f32,
}

impl Pt {
    pub const fn new(t: u32, v: f32) -> Self {
        Self { t, v, c: 0.0 }
    }
}

/// One envelope: points sorted by tick (two on one tick make a jump).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Envelope {
    pub points: Vec<Pt>,
}

fn to_auto(p: &[Pt]) -> Vec<AutoPoint> {
    p.iter().map(|p| AutoPoint { tick: p.t, value: p.v, curve: p.c }).collect()
}

impl Envelope {
    /// The points folded by a multiply-hash (about 1 ns each; every field
    /// counts). `content_hash` and the lanes' draw cache use it: both run
    /// often (every arrangement change, every drawn frame; B5-D14).
    pub fn points_hash(&self) -> u64 {
        let mut f: u64 = 0xcbf2_9ce4_8422_2325 ^ self.points.len() as u64;
        for p in &self.points {
            for w in [p.t as u64, p.v.to_bits() as u64, p.c.to_bits() as u64] {
                f = (f ^ w).wrapping_mul(0x0100_0000_01b3).rotate_left(29);
            }
        }
        f
    }

    pub fn new(points: Vec<Pt>) -> Self {
        let mut e = Self { points };
        e.sort();
        e
    }

    fn sort(&mut self) {
        // Stable: two points on one tick keep their order (a jump).
        self.points.sort_by_key(|p| p.t);
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The value at tick `t` (`None`: no points).
    pub fn value_at(&self, t: f64) -> Option<f32> {
        // In place: the lanes draw ask once per pixel column (B5-D14).
        chip::auto::eval_by(&self.points, t, |p| AutoPoint { tick: p.t, value: p.v, curve: p.c })
    }

    /// Adds a point; returns its index. On a tick that already has points
    /// it goes after them.
    pub fn add(&mut self, t: u32, v: f32) -> usize {
        let i = self.points.partition_point(|p| p.t <= t);
        self.points.insert(i, Pt::new(t, v));
        i
    }

    /// Removes point `i` (its neighbours join).
    pub fn delete(&mut self, i: usize) {
        if i < self.points.len() {
            self.points.remove(i);
        }
    }

    /// Moves point `i` to (`t`, `v`), kept between its neighbours' ticks
    /// (a point never passes another, as in the reference). Returns where it
    /// ended up.
    pub fn move_to(&mut self, i: usize, t: u32, v: f32) -> (u32, f32) {
        let lo = if i > 0 { self.points[i - 1].t } else { 0 };
        let hi = self.points.get(i + 1).map_or(u32::MAX, |p| p.t);
        let p = &mut self.points[i];
        p.t = t.clamp(lo, hi);
        p.v = v;
        (p.t, p.v)
    }

    /// Bends the segment that starts at point `i` (−1 … 1; 0 straight).
    pub fn set_curve(&mut self, i: usize, c: f32) {
        if let Some(p) = self.points.get_mut(i) {
            p.c = c.clamp(-0.95, 0.95);
        }
    }

    /// The segment containing tick `t`: the index of its first point.
    pub fn segment_at(&self, t: f64) -> Option<usize> {
        let i = self.points.partition_point(|p| (p.t as f64) <= t);
        (i > 0 && i < self.points.len()).then_some(i - 1)
    }

    /// Makes the envelope hold its current values at `t0` and `t1` (points
    /// added there if missing), so an edit between them leaves the outside
    /// unchanged.
    fn pin(&mut self, t0: u32, t1: u32) {
        if self.points.is_empty() {
            return;
        }
        for t in [t0, t1] {
            if !self.points.iter().any(|p| p.t == t) {
                let v = self.value_at(t as f64).unwrap_or(0.0);
                self.add(t, v);
            }
        }
    }

    /// Removes the points strictly inside `t0 .. t1`, keeping the values at
    /// both edges (the envelope runs straight between them).
    pub fn delete_range(&mut self, t0: u32, t1: u32) {
        if t1 <= t0 {
            return;
        }
        self.pin(t0, t1);
        // Keep the last point on t0 and the first on t1.
        let first_t0 = self.points.iter().rposition(|p| p.t == t0);
        let last_t1 = self.points.iter().position(|p| p.t == t1);
        let mut k = 0;
        self.points.retain(|p| {
            let keep = p.t < t0 || p.t > t1 || (p.t == t0 && Some(k) == first_t0) || (p.t == t1 && Some(k) == last_t1);
            k += 1;
            keep
        });
        // A straight segment between the edges.
        if let Some(i) = self.points.iter().rposition(|p| p.t == t0) {
            self.points[i].c = 0.0;
        }
    }

    /// The points within `t0 ..= t1` as a clip of envelope relative to `t0`,
    /// with the values at both edges (copy).
    pub fn copy_range(&self, t0: u32, t1: u32) -> Vec<Pt> {
        if self.points.is_empty() || t1 <= t0 {
            return Vec::new();
        }
        let mut e = self.clone();
        e.pin(t0, t1);
        e.points.iter().filter(|p| p.t >= t0 && p.t <= t1).map(|p| Pt { t: p.t - t0, ..*p }).collect()
    }

    /// Replaces `at .. at + len` with `clip` (relative ticks, as `copy_range`
    /// gives), keeping the envelope outside it (paste).
    pub fn paste(&mut self, at: u32, len: u32, clip: &[Pt]) {
        if clip.is_empty() {
            return;
        }
        let end = at.saturating_add(len);
        if !self.points.is_empty() {
            self.pin(at, end);
            self.points.retain(|p| p.t < at || p.t > end);
            // The outside's edge values stay; the pasted values meet them
            // with a jump at each edge.
            let before: Vec<Pt> = self.points.iter().filter(|p| p.t < at).copied().collect();
            let after: Vec<Pt> = self.points.iter().filter(|p| p.t > end).copied().collect();
            let edge_in = self.value_at(at as f64);
            let edge_out = self.value_at(end as f64);
            self.points = before;
            if let Some(v) = edge_in {
                self.points.push(Pt::new(at, v));
            }
            self.points.extend(clip.iter().map(|p| Pt { t: p.t.saturating_add(at), ..*p }));
            if let Some(v) = edge_out {
                self.points.push(Pt::new(end, v));
            }
            self.points.extend(after);
        } else {
            self.points = clip.iter().map(|p| Pt { t: p.t.saturating_add(at), ..*p }).collect();
        }
        self.sort();
        self.dedup();
    }

    /// Draw Mode: the cells `t0 .. t1` of a `grid`-tick grid become flat
    /// steps at `value(cell start)`. The envelope outside is kept.
    pub fn draw_steps(&mut self, t0: u32, t1: u32, grid: u32, value: impl Fn(u32) -> f32) {
        let grid = grid.max(1);
        let a = t0 / grid * grid;
        let b = t1.div_ceil(grid).max(a / grid + 1) * grid;
        let mut clip = Vec::new();
        let mut t = a;
        while t < b {
            let v = value(t);
            clip.push(Pt::new(t - a, v));
            clip.push(Pt::new(t + grid - a, v));
            t += grid;
        }
        // Inside the drawn cells only the steps; their edges jump to the
        // outside's values.
        self.paste(a, b - a, &clip);
    }

    /// Moves the points of `t0 .. t1` by `dt` ticks, replacing what was at
    /// the destination (a clip moved with its automation, or a time
    /// selection dragged).
    pub fn shift_range(&mut self, t0: u32, t1: u32, dt: i64) {
        if dt == 0 || self.points.is_empty() {
            return;
        }
        let clip = self.copy_range(t0, t1);
        self.delete_range(t0, t1);
        let at = (t0 as i64 + dt).max(0) as u32;
        self.paste(at, t1 - t0, &clip);
    }

    /// Drops repeated identical points (same tick and value).
    fn dedup(&mut self) {
        self.points.dedup_by(|b, a| a.t == b.t && a.v == b.v);
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }
}

/// Every envelope of the song, plus the per-session state around them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Automation {
    /// The document: envelopes by owner and parameter.
    pub envs: BTreeMap<AutoKey, Arc<Envelope>>,
    /// Lanes shown under each track (registry ids, in order), and Main's:
    /// saved with the project (not undoable, as in the reference).
    pub shown: BTreeMap<AutoOwner, Vec<String>>,
    /// Overridden by hand during playback (not saved): the manual value
    /// plays until Re-Enable Automation.
    pub overridden: BTreeSet<AutoKey>,
}

impl Automation {
    pub fn get(&self, k: &AutoKey) -> Option<&Envelope> {
        self.envs.get(k).map(|e| &**e).filter(|e| !e.is_empty())
    }

    /// The envelope for `k`, created empty if missing (copy on write).
    pub fn edit(&mut self, k: &AutoKey) -> &mut Envelope {
        Arc::make_mut(self.envs.entry(k.clone()).or_default())
    }

    /// Drops empty envelopes (after an edit removed every point).
    pub fn prune(&mut self) {
        self.envs.retain(|_, e| !e.is_empty());
        let keys: Vec<AutoKey> = self.envs.keys().cloned().collect();
        self.overridden.retain(|k| keys.contains(k));
    }

    /// Whether `owner`'s parameter `id` has automation.
    pub fn has(&self, owner: AutoOwner, id: &str) -> bool {
        self.get(&(owner, id.to_string())).is_some()
    }

    /// The parameters of `owner` with automation, in key order.
    pub fn automated(&self, owner: AutoOwner) -> Vec<String> {
        self.envs.iter().filter(|((o, _), e)| *o == owner && !e.is_empty()).map(|((_, id), _)| id.clone()).collect()
    }

    /// A track was deleted: its envelopes and lanes go.
    pub fn remove_owner(&mut self, owner: AutoOwner) {
        self.envs.retain(|(o, _), _| *o != owner);
        self.shown.remove(&owner);
        self.overridden.retain(|(o, _)| *o != owner);
    }

    /// Overrides `k` (a control moved by hand during playback, not armed).
    /// Returns whether it changed.
    pub fn override_key(&mut self, k: &AutoKey) -> bool {
        self.get(k).is_some() && self.overridden.insert(k.clone())
    }

    /// Re-Enable Automation: every overridden control follows its envelope
    /// again.
    pub fn re_enable(&mut self) -> bool {
        let any = !self.overridden.is_empty();
        self.overridden.clear();
        any
    }

    /// A clip on track `owner` moved from `start .. start + len` by `dt`
    /// ticks, onto track `to`: its automation goes with it (Lock Envelopes
    /// off; manual 26.9.2). On another track, each envelope moves to the same
    /// parameter there.
    pub fn carry(&mut self, from: AutoOwner, to: AutoOwner, start: u32, len: u32, dt: i64) {
        if dt == 0 && from == to {
            return;
        }
        let keys: Vec<AutoKey> = self.envs.keys().filter(|(o, _)| *o == from).cloned().collect();
        for k in keys {
            let end = start.saturating_add(len);
            if from == to {
                self.edit(&k).shift_range(start, end, dt);
            } else {
                let clip = self.edit(&k).copy_range(start, end);
                self.edit(&k).delete_range(start, end);
                let at = (start as i64 + dt).max(0) as u32;
                self.edit(&(to, k.1.clone())).paste(at, len, &clip);
            }
        }
        self.prune();
    }

    /// The registry parameter for an owner's lane (`None`: the owner has no
    /// such parameter now, e.g. a locked track's fader).
    pub fn param_of(lanes: &[Lane], owner: AutoOwner, id: &str) -> Option<Param> {
        registry_for(lanes, owner)?.into_iter().find(|p| p.id == id)
    }

    /// The engine's envelopes for one track (by id), and their points: only
    /// parameters the track has now, never an overridden one.
    pub fn engine_track(&self, lane: &Lane) -> (Vec<AutoEnv>, Vec<AutoPoint>) {
        let owner = AutoOwner::Track(lane.id);
        let reg = params::registry(Owner::Track { kind: lane.voice, locked: lane.lock.is_some() });
        let mut envs = Vec::new();
        let mut pts = Vec::new();
        for ((o, id), e) in &self.envs {
            if *o != owner || e.is_empty() || self.overridden.contains(&(*o, id.clone())) {
                continue;
            }
            let Some(p) = reg.iter().find(|p| p.id == id) else { continue };
            let Some(target) = target_of(p.id, lane.voice) else { continue };
            envs.push(AutoEnv { target, start: pts.len() as u32, len: e.points.len() as u32 });
            pts.extend(to_auto(&e.points));
        }
        (envs, pts)
    }

    /// The Main gain's envelope for the engine (dB), unless overridden.
    pub fn engine_main(&self) -> Vec<AutoPoint> {
        let k = (AutoOwner::Main, "main.gain".to_string());
        match self.get(&k) {
            Some(e) if !self.overridden.contains(&k) => to_auto(&e.points),
            _ => Vec::new(),
        }
    }

    /// A hash of what the engine hears (envelopes and overrides): the song
    /// snapshot is rebuilt when it changes.
    pub fn content_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for ((o, id), e) in &self.envs {
            (o, id, e.points.len()).hash(&mut h);
            // Not SipHash per point: it runs on every arrangement change.
            e.points_hash().hash(&mut h);
        }
        self.overridden.hash(&mut h);
        h.finish()
    }

    /// Automation Mode's first look: a track showing no lane gets one, its
    /// first automated parameter, else its first parameter (the fader on a
    /// studio track, a channel control on a locked one).
    pub fn default_lanes(&mut self, lanes: &[Lane]) {
        for l in lanes {
            let owner = AutoOwner::Track(l.id);
            let reg = registry_for(lanes, owner).unwrap_or_default();
            let shown = self.shown.entry(owner).or_default();
            shown.retain(|id| reg.iter().any(|p| p.id == id.as_str()));
            if shown.is_empty() {
                let auto: Vec<String> = self.envs.iter().filter(|((o, _), e)| *o == owner && !e.is_empty()).map(|((_, id), _)| id.clone()).filter(|id| reg.iter().any(|p| p.id == id.as_str())).collect();
                if let Some(first) = auto.into_iter().next().or_else(|| reg.first().map(|p| p.id.to_string())) {
                    self.shown.insert(owner, vec![first]);
                }
            }
        }
    }

    /// Every point of every envelope (perf and tests).
    pub fn point_count(&self) -> usize {
        self.envs.values().map(|e| e.len()).sum()
    }
}

/// The registry id of a channel control.
pub fn chip_param_id(p: ParamId) -> &'static str {
    match p {
        ParamId::Duty => "chip.duty",
        ParamId::Volume => "chip.volume",
        ParamId::Envelope => "chip.envelope",
        ParamId::Sweep => "chip.sweep",
        ParamId::SweepRate => "chip.sweep_rate",
        ParamId::NoiseMode => "chip.noise_mode",
        ParamId::NoisePeriod => "chip.noise_period",
        ParamId::Sample => "chip.sample",
        ParamId::Rate => "chip.rate",
        ParamId::Loop => "chip.loop",
    }
}

/// The automated-control marker's state: `None` (no automation), or
/// whether it is overridden.
pub fn mark(a: &crate::state::Arrangement, owner: AutoOwner, id: &str) -> Option<bool> {
    // No envelopes (most songs): no key built (rows ask on every draw).
    if a.automation.envs.is_empty() {
        return None;
    }
    let k = (owner, id.to_string());
    a.automation.get(&k).map(|_| a.automation.overridden.contains(&k))
}

/// Draws the marker: a small white dot (grey while overridden) centred at
/// `c` (device px). The reference's LED is red; ours is neither red nor lime.
pub fn draw_mark(canvas: &vizia::vg::Canvas, c: (f32, f32), s: f32, overridden: bool) {
    let col = if overridden { crate::theme::tokens::AUTO_OVERRIDDEN } else { crate::theme::tokens::AUTO_MARK };
    canvas.draw_circle(c, crate::theme::tokens::AUTO_MARK_D * 0.5 * s, &crate::views::gfx::fill(col));
}

/// Whether a track of `kind` (locked or not) has parameter `id`: the
/// registry's rule without building it (no allocation: the arrangement asks
/// for every row on every frame in Automation Mode). Kept equal to
/// `params::registry` by `valid_for_matches_the_registry`.
pub fn valid_for(id: &str, kind: chip::voice::VoiceKind, locked: bool) -> bool {
    use chip::voice::VoiceKind as K;
    match id {
        "track.gain" | "track.pan" | "track.active" | "voice.unison" | "voice.detune" | "voice.drift" => !locked,
        "macro.arpeggio" | "macro.pitch" | "macro.vibrato" => true,
        "macro.volume" => kind != K::Triangle,
        "macro.gate" => kind == K::Triangle,
        "macro.duty" => matches!(kind, K::Pulse | K::Vrc6Pulse | K::Noise),
        id if id.starts_with("chip.") => target_of(id, kind).is_some(),
        _ => false,
    }
}

/// An owner's registry (`None`: a track that doesn't exist).
pub fn registry_for(lanes: &[Lane], owner: AutoOwner) -> Option<Vec<Param>> {
    match owner {
        AutoOwner::Main => Some(params::registry(Owner::Main)),
        AutoOwner::Track(id) => {
            let l = lanes.iter().find(|l| l.id == id)?;
            Some(params::registry(Owner::Track { kind: l.voice, locked: l.lock.is_some() }))
        }
    }
}

/// The engine target of a registry id on a track of `kind`.
pub fn target_of(id: &str, kind: chip::voice::VoiceKind) -> Option<AutoTarget> {
    let chip_param = |p: ParamId| ParamId::for_lane(kind.lane()).contains(&p).then_some(AutoTarget::Chip(p));
    Some(match id {
        "track.gain" => AutoTarget::Gain,
        "track.pan" => AutoTarget::Pan,
        "track.active" => AutoTarget::Active,
        "chip.duty" => return chip_param(ParamId::Duty),
        "chip.volume" => return chip_param(ParamId::Volume),
        "chip.envelope" => return chip_param(ParamId::Envelope),
        "chip.sweep" => return chip_param(ParamId::Sweep),
        "chip.sweep_rate" => return chip_param(ParamId::SweepRate),
        "chip.noise_mode" => return chip_param(ParamId::NoiseMode),
        "chip.noise_period" => return chip_param(ParamId::NoisePeriod),
        "chip.sample" => return chip_param(ParamId::Sample),
        "chip.rate" => return chip_param(ParamId::Rate),
        "chip.loop" => return chip_param(ParamId::Loop),
        "macro.volume" => AutoTarget::MacroVolume,
        "macro.arpeggio" => AutoTarget::MacroArp,
        "macro.pitch" => AutoTarget::MacroPitch,
        "macro.vibrato" => AutoTarget::MacroVibrato,
        "macro.duty" => AutoTarget::MacroDuty,
        "macro.gate" => AutoTarget::MacroGate,
        "voice.unison" => AutoTarget::Unison,
        "voice.detune" => AutoTarget::Detune,
        "voice.drift" => AutoTarget::Drift,
        "main.gain" => AutoTarget::MainGain,
        _ => return None,
    })
}

/// A parameter's value as a 0 … 1 position in its lane (the fader in dB
/// from the floor).
pub fn norm(p: &Param, v: f32) -> f32 {
    let lo = if p.min.is_finite() { p.min } else { DB_FLOOR };
    ((v - lo) / (p.max - lo).max(1e-6)).clamp(0.0, 1.0)
}

/// The inverse of `norm`; stepped parameters snap to their steps.
pub fn denorm(p: &Param, x: f32) -> f32 {
    let lo = if p.min.is_finite() { p.min } else { DB_FLOOR };
    let v = lo + x.clamp(0.0, 1.0) * (p.max - lo);
    if p.stepped {
        v.round()
    } else {
        v
    }
}

/// How the lane draws a value that plays: locked tracks' parameters are
/// quantised per frame, so their lanes show the steps the chip plays.
pub fn quantised(p: &Param, v: f32) -> f32 {
    if p.stepped {
        chip::auto::quantise(v, p.min as i32, p.max as i32) as f32
    } else {
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(p: &[(u32, f32)]) -> Envelope {
        Envelope::new(p.iter().map(|&(t, v)| Pt::new(t, v)).collect())
    }

    fn ticks(e: &Envelope) -> Vec<(u32, f32)> {
        e.points.iter().map(|p| (p.t, p.v)).collect()
    }

    /// AU-01: add, move, delete and curve breakpoints.
    #[test]
    fn au01_add_move_delete_and_curve() {
        let mut e = env(&[(0, 0.0), (960, 10.0)]);
        let i = e.add(480, 2.0);
        assert_eq!(i, 1);
        assert_eq!(e.value_at(480.0), Some(2.0));
        // A point never passes its neighbours.
        assert_eq!(e.move_to(1, 5000, 4.0), (960, 4.0));
        assert_eq!(e.move_to(1, 700, 6.0), (700, 6.0));
        assert_eq!(e.value_at(700.0), Some(6.0));
        // Curve: the segment from point 0 bends; the ends stay.
        let straight = e.value_at(350.0).unwrap();
        e.set_curve(0, 0.8);
        let bent = e.value_at(350.0).unwrap();
        assert!(bent > straight + 1.0, "{straight} -> {bent}");
        assert_eq!(e.value_at(700.0), Some(6.0));
        assert_eq!(e.segment_at(350.0), Some(0));
        // Delete: the neighbours join.
        e.delete(1);
        assert_eq!(ticks(&e), vec![(0, 0.0), (960, 10.0)]);
    }

    /// AU-02: Draw Mode draws flat steps at the grid; outside is kept.
    #[test]
    fn au02_draw_mode_draws_steps_at_the_grid() {
        let mut e = env(&[(0, 0.0), (3840, 0.0)]);
        e.draw_steps(500, 1400, 240, |t| (t / 240) as f32);
        // Cells 480..720 (2), 720..960 (3), … 1200..1440 (5).
        for (t, want) in [(500.0, 2.0), (719.0, 2.0), (720.0, 3.0), (1300.0, 5.0)] {
            assert_eq!(e.value_at(t), Some(want), "at {t}");
        }
        assert_eq!(e.value_at(100.0), Some(0.0), "before: kept");
        assert_eq!(e.value_at(3000.0), Some(0.0), "after: kept");
    }

    /// AU-03: a time selection's envelope copies, pastes and deletes.
    #[test]
    fn au03_time_selection_copy_paste_delete() {
        let mut e = env(&[(0, 0.0), (960, 8.0), (1920, 0.0), (3840, 0.0)]);
        let clip = e.copy_range(480, 1440);
        assert_eq!(clip.first().map(|p| (p.t, p.v)), Some((0, 4.0)));
        assert_eq!(clip.last().map(|p| (p.t, p.v)), Some((960, 4.0)));
        e.paste(2400, 960, &clip);
        assert_eq!(e.value_at(2400.0 + 480.0), Some(8.0), "the peak pasted");
        assert_eq!(e.value_at(3600.0), Some(0.0), "after the paste: kept");
        e.delete_range(480, 1440);
        assert_eq!(e.value_at(960.0), Some(4.0), "straight between the edges");
        assert_eq!(e.value_at(300.0), Some(2.5), "before: kept");
    }

    /// AU-04: a moved clip carries its automation (Lock Envelopes off), and
    /// to another track; a time-selection move does the same.
    #[test]
    fn au04_clips_carry_their_automation() {
        let k = (AutoOwner::Track(1), "chip.volume".to_string());
        let mut a = Automation::default();
        *a.edit(&k) = env(&[(0, 15.0), (960, 15.0), (1440, 3.0), (1920, 15.0)]);
        a.carry(AutoOwner::Track(1), AutoOwner::Track(1), 960, 960, 3840);
        let e = a.get(&k).unwrap();
        assert_eq!(e.value_at(960.0 + 3840.0 + 480.0), Some(3.0), "the dip moved with the clip");
        assert_eq!(e.value_at(1440.0), Some(15.0), "and left its old place");
        // Onto another track.
        a.carry(AutoOwner::Track(1), AutoOwner::Track(2), 4800, 960, -4800);
        let e2 = a.get(&(AutoOwner::Track(2), "chip.volume".into())).unwrap();
        assert_eq!(e2.value_at(480.0), Some(3.0));
    }

    /// AU-06: override and Re-Enable: an overridden envelope stays in the
    /// document but doesn't reach the engine until Re-Enable.
    #[test]
    fn au06_override_and_re_enable() {
        let mut lane = crate::state::Lane::studio(5, chip::voice::VoiceKind::Pulse, "T".into(), crate::theme::tokens::Rgba::hex(0x4FA8E0));
        lane.lock = None;
        let k = (AutoOwner::Track(5), "track.gain".to_string());
        let mut a = Automation::default();
        *a.edit(&k) = env(&[(0, 0.0), (960, -12.0)]);
        assert_eq!(a.engine_track(&lane).0.len(), 1);
        assert!(a.override_key(&k));
        assert_eq!(a.engine_track(&lane).0.len(), 0, "overridden: the manual value plays");
        assert!(a.get(&k).is_some(), "the envelope is kept");
        assert!(a.re_enable());
        assert_eq!(a.engine_track(&lane).0.len(), 1);
        assert!(!a.override_key(&(AutoOwner::Track(5), "track.pan".into())), "nothing to override without an envelope");
    }

    /// AU-08: a locked track has no fader or pan lane (the envelope is kept,
    /// unheard), and no lane for a parameter its channel lacks.
    #[test]
    fn au08_locked_tracks_have_no_fader_or_pan_lanes() {
        use chip::voice::VoiceKind;
        let mut lane = crate::state::Lane::studio(3, VoiceKind::Triangle, "T".into(), crate::theme::tokens::Rgba::hex(0x4FA8E0));
        let mut a = Automation::default();
        *a.edit(&(AutoOwner::Track(3), "track.gain".into())) = env(&[(0, -6.0)]);
        *a.edit(&(AutoOwner::Track(3), "macro.gate".into())) = env(&[(0, 1.0)]);
        lane.lock = None;
        assert_eq!(a.engine_track(&lane).0.len(), 2, "studio: fader and gate");
        lane.lock = Some(chip::driver::LANE_TRI);
        let (envs, _) = a.engine_track(&lane);
        assert_eq!(envs.len(), 1, "locked: the gate only");
        assert_eq!(envs[0].target, AutoTarget::MacroGate);
        let lanes = vec![lane];
        assert!(Automation::param_of(&lanes, AutoOwner::Track(3), "track.gain").is_none());
        assert!(Automation::param_of(&lanes, AutoOwner::Track(3), "track.pan").is_none());
        assert!(Automation::param_of(&lanes, AutoOwner::Track(3), "chip.duty").is_none(), "the triangle has no duty");
        assert!(Automation::param_of(&lanes, AutoOwner::Track(3), "macro.volume").is_none(), "nor a volume");
        assert!(a.has(AutoOwner::Track(3), "track.gain"), "kept for an unlock");
    }

    #[test]
    fn valid_for_matches_the_registry() {
        let ids = [
            "track.gain", "track.pan", "track.active", "chip.duty", "chip.volume", "chip.envelope", "chip.sweep", "chip.sweep_rate", "chip.noise_mode", "chip.noise_period", "chip.sample", "chip.rate", "chip.loop",
            "macro.volume", "macro.arpeggio", "macro.pitch", "macro.vibrato", "macro.duty", "macro.gate", "voice.unison", "voice.detune", "voice.drift", "main.gain",
        ];
        for kind in chip::voice::VoiceKind::ALL {
            for locked in [false, true] {
                let reg = params::registry(Owner::Track { kind, locked });
                for id in ids {
                    assert_eq!(valid_for(id, kind, locked), reg.iter().any(|p| p.id == id), "{kind:?} locked {locked} {id}");
                }
            }
        }
    }

    #[test]
    fn norm_and_denorm_round_trip_and_snap() {
        let reg = params::registry(Owner::Track { kind: chip::voice::VoiceKind::Pulse, locked: true });
        let duty = reg.iter().find(|p| p.id == "chip.duty").unwrap();
        assert_eq!(denorm(duty, norm(duty, 2.0)), 2.0);
        assert_eq!(denorm(duty, 0.4), 1.0, "snaps to a step");
        let gain = &params::registry(Owner::Main)[0];
        assert_eq!(norm(gain, f32::NEG_INFINITY), 0.0);
        assert_eq!(denorm(gain, 0.0), DB_FLOOR);
    }
}
