//! TAP tempo (design pass §3.3): each tap sets the tempo from the average
//! of the last four intervals; a tap more than two seconds after the
//! previous one starts over. One tap sequence is one undo step (the app
//! commits the first change of a sequence and amends that step after).

use std::time::{Duration, Instant};

/// Taps further apart than this start a new sequence.
pub const RESTART_AFTER: Duration = Duration::from_secs(2);
/// The tempo is the average of at most this many intervals.
pub const INTERVALS: usize = 4;

#[derive(Debug, Default)]
pub struct TapTempo {
    /// The current sequence's taps, newest last (at most INTERVALS + 1).
    taps: Vec<Instant>,
}

/// What one tap did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tap {
    /// The new tempo, once there are two taps in the sequence.
    pub bpm: Option<f32>,
    /// This tap started a new sequence.
    pub restarted: bool,
}

impl TapTempo {
    pub fn tap(&mut self, now: Instant) -> Tap {
        let restarted = self.taps.last().is_none_or(|&t| now.saturating_duration_since(t) > RESTART_AFTER);
        if restarted {
            self.taps.clear();
        }
        self.taps.push(now);
        if self.taps.len() > INTERVALS + 1 {
            self.taps.remove(0);
        }
        let bpm = (self.taps.len() >= 2).then(|| {
            let span = self.taps[self.taps.len() - 1].duration_since(self.taps[0]).as_secs_f64();
            let avg = span / (self.taps.len() - 1) as f64;
            (60.0 / avg.max(1e-3)) as f32
        });
        Tap { bpm, restarted }
    }

    /// The sequence is still open (the next tap would continue it).
    pub fn open(&self, now: Instant) -> bool {
        self.taps.last().is_some_and(|&t| now.saturating_duration_since(t) <= RESTART_AFTER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_taps_at_500ms_is_120() {
        let mut t = TapTempo::default();
        let t0 = Instant::now();
        let r = t.tap(t0);
        assert_eq!(r, Tap { bpm: None, restarted: true });
        let mut last = None;
        for k in 1..4 {
            let r = t.tap(t0 + Duration::from_millis(500 * k));
            assert!(!r.restarted);
            last = r.bpm;
        }
        assert!((last.unwrap() - 120.0).abs() < 0.01, "{last:?}");
    }

    #[test]
    fn averages_the_last_four_intervals() {
        let mut t = TapTempo::default();
        let t0 = Instant::now();
        // A slow start (1 s), then four 500 ms intervals: only those count.
        let mut at = t0;
        t.tap(at);
        at += Duration::from_millis(1000);
        t.tap(at);
        let mut bpm = 0.0;
        for _ in 0..4 {
            at += Duration::from_millis(500);
            bpm = t.tap(at).bpm.unwrap();
        }
        assert!((bpm - 120.0).abs() < 0.01, "{bpm}");
    }

    #[test]
    fn a_long_pause_starts_over() {
        let mut t = TapTempo::default();
        let t0 = Instant::now();
        t.tap(t0);
        t.tap(t0 + Duration::from_millis(400));
        let r = t.tap(t0 + Duration::from_millis(2500));
        assert_eq!(r, Tap { bpm: None, restarted: true });
        assert!(t.open(t0 + Duration::from_millis(3000)));
        assert!(!t.open(t0 + Duration::from_millis(4600)));
    }
}
