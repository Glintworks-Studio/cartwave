# ENGINE: real-time NES + VRC6 chip engine in the prototype UI

**Question (ENGINE_PROMPT.md §1):** can Rust + vizia + a real-time chip engine play authentic
NES + VRC6 audio glitch-free while the UI is busy, and stay responsive to edits during playback?

**Answer: yes, on this machine, with margin.** WASAPI loopback ground truth recorded **0 dropouts**
in all 15 stress runs (3.5 hours of playback). That includes a 250 ms UI stall every 2 s, ~200 new
200-clip snapshots per second for 10 minutes, and window churn. The audio callback costs **~0.2%
of its buffer period** (p99 ≤ 0.6%, max ≤ 3.5%). Every edit is heard at the next 60.1 Hz frame
tick after it reaches the engine (≤ 16.64 ms). The picture is within 0.3 ms of the sound. Switching
the default output leaves a ~0.4 s gap and recovers by itself. The verdict, with conditions, is in
§9.

**Machine:** Windows 11 Pro 26200 · AMD Ryzen 7 9800X3D · RTX 4070 SUPER · MOTU M Series USB
interface (WASAPI shared mode) · 60 Hz display · release builds · rustc 1.100.0.

Contents:

1. Architecture
2. The emulator (`crates/chip`)
3. Band-limiting and the output stage
4. The sound driver
5. Real-time host and UI wiring
6. Authenticity checks and known gaps
7. Stress tests (§5 of the brief)
8. Windows audio findings, and what will differ on macOS
9. Verdict

Run: `cargo run --release -p app`. **F7** cycles the output filter (NES, NES + 440 Hz, Famicom,
off). The F3 overlay shows the audio device, latency, callback cost and underruns. The other keys
are as in FINDINGS.md.

---

## 1. Architecture

```
 UI thread (vizia, PERF.md pacing)          audio thread (cpal / WASAPI)              host thread
 ┌───────────────────────────────┐  commands  ┌──────────────────────────────────┐    ┌────────────────────┐
 │ AppState                      │  rtrb SPSC │ Core::process (NoAllocGuard)     │    │ owns cpal::Stream  │
 │  Arrangement (the model)      │───────────▶│  1. drain commands (wait-free)   │    │ builds / rebuilds  │
 │  edit → overlap rule          │ Box<Song-  │  2. Player.render(song)          │    │ on device errors   │
 │  → SongSnapshot (built here)  │  Snapshot> │     driver ticks at 60.0988 Hz   │    │ (cpal default-     │
 │                               │            │     2A03 + VRC6 event-driven     │    │  device monitor)   │
 │  collect_garbage() frees      │◀───────────│     band-limited steps, filter   │    │                    │
 │  old snapshots here           │  garbage   │  3. old snapshot → garbage queue │    │ CoreCell: atomic   │
 │                               │  rtrb SPSC │  4. publish (atomics only)       │    │ swap hands the     │
 │  each frame reads Shared:     │◀ ─ ─ ─ ─ ─ │     seqlock position + latency,  │    │ Core from stream   │
 │  playhead (latency-comp.),    │   Shared   │     levels (max-merge), note-ons │    │ to stream          │
 │  meters, lights, stats        │  (atomics) │     callback stats, underruns    │    └────────────────────┘
 └───────────────────────────────┘            └──────────────────────────────────┘
```

**Who owns what**

| Data | Owner | Crosses threads as |
|---|---|---|
| Arrangement (clips, notes, selection, view) | UI thread | never; the engine gets snapshots |
| `SongSnapshot` (per-lane sorted, non-overlapping notes) | built and freed on the UI thread; owned by `Core` while current | `Box` moved through the command queue; returned through the garbage queue |
| `Player` (driver + `Synth` + transport) | `Core` (audio thread) | never |
| `Core` | `CoreCell` (`Arc<AtomicPtr>`) | taken with a wait-free swap by each callback; by the host only after the stream is dropped (cpal joins the audio thread) |
| Transport, parameters, mute/solo, chip, filter | UI thread (mirror) | `Command`s (plain values) |
| Position, levels, note-ons, statistics | `Shared` (`Arc`, atomics) | written by the audio thread, read by the UI on its own frame tick |

**Audio-thread rules, and how they are enforced**

- **No locks.** The queues are `rtrb` SPSC rings, publishing is atomics plus a seqlock, and the core
  is handed over with an atomic swap.
- **No allocation or freeing.**
  - `Core::process` runs under `NoAllocGuard`. The app installs `CheckingAllocator` globally:
    debug builds abort on a violation, release builds count it, and every stress log shows the
    count. The engine's unit tests run `process` under the checker as well, with a test proving the
    guard fires.
  - A new snapshot waits in the queue while the garbage ring is full, so nothing is ever dropped on
    the audio thread.
- **No I/O, no logging, no syscalls.** The only OS calls are `Instant::now()` and
  `GetCurrentThreadId()`. On Windows, `QueryPerformanceCounter` is a user-mode read, and cpal calls
  it on this thread anyway; `GetCurrentThreadId` is a TEB read.
- **The audio thread never calls the UI.** The UI reads `Shared` during its existing pump tick.
  There are no new wakeups: the PERF.md pacing and the frame cap are unchanged, and the host thread
  parks until an error unparks it.

**Crates.** `crates/chip` is `no_std`, has no dependencies and no allocation after construction.
`crates/engine` depends on `cpal` 0.18.2 (Apache-2.0) and `rtrb` 0.4.1 (MIT/Apache), both approved
for this task. Their transitive runtime dependencies are all MIT/Apache/BSD (`cargo tree`: see §8).

## 2. The emulator (`crates/chip`)

Clean-room, written only from the NESdev wiki pages below. No emulator source, including the
earlier C++ one, was read. Every channel is a register-driven state machine clocked from the NTSC
CPU clock. **NTSC CPU clock = 236.25 MHz / 11 / 12 = 1 789 772.73 Hz** (CPU page and Cycle
reference chart agree). The video frame is 29 780.5 CPU cycles (60.0988 Hz).

| Unit | Implemented (file) | Tested against |
|---|---|---|
| Pulse ×2 | `pulse.rs`: 4 duty sequences, timer = (t+1) APU cycles = 2(t+1) CPU cycles, envelope, length counter, sweep with pulse 1 ones' / pulse 2 two's complement negate. Muting when t < 8 or target > $7FF, **also while the sweep is disabled** (wiki) | duty waveforms and shares, f = CPU/(16(t+1)), phase reset on $4003, every muting threshold, sweep update cadence |
| Envelope / length counter | `units.rs` | decay = 15(V+1) quarter frames, loop, constant volume, the full length table and its structure, halt, disable clears |
| Triangle | `triangle.rs`: 32-step sequencer clocked every CPU cycle, linear counter with the control/reload-flag rules, gated by linear and length | sequence order and period, f = CPU/(32(t+1)), the reload-flag rules incl. the "$80 halt" idiom |
| Noise | `noise.rs`: 15-bit LFSR (bit 0 XOR bit 1 or bit 6), NTSC period table, envelope, length; LFSR = 1 at power-up | long loop = 32 767 steps; short mode = 93 or 31 steps from every reachable state; first 20 outputs vs a bit-level model; period table; muting |
| DMC | `dmc.rs`: output unit (±2, clamped, LSB first, silence flag), memory reader (address wrap $FFFF→$8000, loop, IRQ), NTSC rate table, $4011 direct load, 16 KiB sample ROM at $C000 | delta steps and clamping, address/length formulas, loop, IRQ, rate table |
| Frame counter | `frame_counter.rs`: 4-step (7457, 14913, 22371, 29829 / 29830) and 5-step (…, 37281 / 37282) at the wiki's APU-cycle times ×2. $4017 reset after 3 / 4 cycles by parity, immediate quarter+half clock in 5-step mode, frame IRQ flag | exact step cycles in both modes, 240 Hz quarter rate, parity delay |
| VRC6 | `vrc6.rs`: 2 pulses (8 duties + "digitized" mode, enable resets the duty phase), saw (6 adds then reset on the 7th accumulator clock, accumulator clocked on every 2nd divider clock, output = top 5 bits), $9003 halt, 16× / 256× (4- / 8-bit period shift) | the wiki's saw trace for A = $08 reproduced step by step, wrap above 42, all 8 duties, frequency formulas incl. the saw's /14, shift and halt |
| Whole system | `synth.rs`: event-driven clocking with sleeping channels (below) | **bit-identical to one-cycle-at-a-time stepping** through sleeps, wakes, halts and 16× shifts (`optimised_equals_reference`) |

**Event-driven, with sleeping channels.**
- `run_until` jumps straight to the next cycle where any timer or the frame counter fires.
- A channel whose output can't change before the next register write or frame clock is taken out
  of the loop: muted pulse, gated triangle, silent noise, disabled VRC6 voice. At the next write or
  frame clock its timer phase is fast-forwarded in closed form.
- Before sleeping existed, an *idle* chip cost more than a busy one, because power-up period-0
  timers fired every cycle: 247 vs 97 ns per output sample. Now idle costs 16 ns.

**Decisions where the documentation is ambiguous or silent** (each chosen deliberately):

| Question | Choice | Why |
|---|---|---|
| Triangle at ultrasonic periods (t = 0, 1) | Emulated exactly, not muted | The wiki says external filtering averages it to ~half amplitude. Band-limiting does exactly that, and keeps the documented pop of the "write period 0 to silence" idiom. Cost: up to 1.68 M events/s (6.3% of a core) while it lasts. Our driver never writes such periods (lowest triangle period it uses: 12) |
| $4017 write delay: which CPU cycles are "APU cycles"? | Even cycles (where our pulse/noise/DMC timers clock) | The wiki defines 3 vs 4 cycles relative to the APU cycle; the absolute phase is unobservable without a CPU |
| Power-up state | Triangle on step 0 (outputs 15), LFSR = 1, everything else 0 | Wiki for the LFSR; the triangle's step is unspecified. Treated as "always was there", so there is no click at engine start |
| Noise $F period | 4068 (not 2046) | 2046 is only on the earliest, recalled 2A03 revisions |
| VRC6 register layout | VRC6a ($x000-$x003) | VRC6b only swaps two address lines on the cartridge; no effect on sound |

## 3. Band-limiting and the output stage

### 3.1 Choice: band-limited steps (own design), not oversample-and-decimate

The mixed output is piecewise constant and changes at CPU-cycle resolution. Each change of size
`d` at time `t` is turned into `d × (band-limited impulse)` at `t`'s exact sub-sample position, and
the buffer is integrated (`blep.rs`).

| | Band-limited steps (chosen) | Run at a high rate + polyphase decimation |
|---|---|---|
| Cost | ∝ number of output **changes** (64 MACs each). A typical song is 20–70 k changes/s | ∝ internal rate × filter length, even when silent (1.79 MHz × taps) |
| Exactness | Edge timing at 1/256-sample resolution, linearly interpolated, effectively continuous | Exact at the internal rate; quality depends on the decimator |
| Character | No smoothing below the cutoff: steps stay steps | Same, given a good decimator |

The kernel is a Kaiser-windowed sinc:
- 32 taps × 256 phases, cutoff 0.45 fs (21.6 kHz), β = 9.
- Taps are 20-bit integers that sum exactly to 2²⁰; amplitudes are 24-bit fixed point. Each step
  is split `d0 + d1 = d` between the two neighbouring phases (linear interpolation).
- The integrator is exact integer arithmetic, so there is **no DC drift**: after 200 buffers of
  toggling, a held level reads exactly 1.0 (tested).
- Latency is 16 samples (0.33 ms), and it is included in the latency compensation.

### 3.2 Measurements (`cargo run --release -p chip --example lab -- alias`)

Non-harmonic power relative to harmonic power in 20 Hz–20 kHz / the strongest non-harmonic
component relative to the fundamental. 48 kHz, 65 536-point Blackman–Harris FFT.

| Tone | f0 | Naive point sampling, raw | **Band-limited, raw** | Naive, NES filter | Band-limited, NES filter |
|---|---|---|---|---|---|
| pulse 50%, t=53 | 2071.5 Hz | −15.7 dB / −23.5 dBc | **−88.3 dB / −94.7 dBc** | −17.3 / −26.4 | −88.4 / −94.7 |
| pulse 12.5%, t=26 | 4143.0 Hz | −8.8 / −12.7 | **−85.3 / −85.9** | −8.8 / −12.6 | −84.4 / −85.7 |
| pulse 25%, t=211 | 527.6 Hz | −20.3 / −31.2 | **−88.5 / −92.6** | −22.2 / −34.7 | −88.6 / −92.6 |
| triangle, t=26 | 2071.5 Hz | −25.9 / −29.7 | **−89.3 / −94.7** | −28.4 / −34.3 | −89.3 / −94.7 |
| VRC6 saw, t=60, A=42 | 2095.8 Hz | −14.5 / −23.0 | **−86.4 / −91.0** | −16.0 / −25.9 | −86.4 / −91.7 |
| VRC6 pulse 3/16, t=101 | 1096.7 Hz | −16.2 / −24.6 | **−87.8 / −93.8** | −17.9 / −26.7 | −87.8 / −93.8 |

How the floor was found, one change at a time (12.5% pulse at 4.1 kHz, the hardest case):

| Step | Aliasing | What limited it |
|---|---|---|
| 256 phases, nearest phase, 15-bit taps | −56.9 dB | Kernel length (64 taps), β (12) and cutoff (0.40) changed nothing (±0.3 dB). A back-of-envelope jitter estimate (±1/512 sample edge-timing error) predicted −52 dB: **phase quantisation** |
| + linear interpolation between phases | −76.1 dB | Now 64 taps made it *worse*: **15-bit tap quantisation** |
| + 20-bit taps | **−85.3 dB** | 512 phases, β 11, or cutoff 0.42 change < 1 dB |

The NES filter barely changes aliasing: its 14 kHz low-pass can't remove aliases that already
folded into the audio band. Pitch: the measured fundamental matches the period formula within
**0.10 cents** for every oscillator (`lab verify`). The tuning *error* is in §4.

**Cost** (`lab cost`, one core, 60 s rendered in 256-sample buffers):

| Case | Output changes / s | ns per output sample | % of real time |
|---|---|---|---|
| Silent chip | 0 | 15.8 | 0.08% |
| All 8 channels busy (A4-ish) | 70 571 | 97.1 | 0.47% |
| Noise at period index 0 (worst noise) | 223 727 | 216.6 | 1.04% |
| Triangle at t = 0 (ultrasonic, worst case) | 1 677 796 | 1320.2 | 6.34% |

### 3.3 Mixing

- **2A03.** The exact non-linear formulas from APU Mixer, evaluated only when an output changes:
  `pulse_out = 95.88 / (8128 / (p1 + p2) + 100)`, and `tnd_out = 159.79 / (1 / (t/8227 + n/12241 + d/22638) + 100)`.
  - The wiki's lookup tables were tested against these formulas. The 31-entry pulse table is within
    0.4% (95.52 vs 95.88).
  - The 203-entry tnd table is within 4% for every combination **except the DMC alone (t = n = 0,
    d = 1..26), which reads up to 4.66% high**. The wiki's own "within 4%" is therefore slightly
    optimistic. Its separate remark that the approximation makes DMC "slightly louder" is the
    explanation.
- **VRC6.** A linear DAC (6-bit sum of two 4-bit pulses and the 5-bit saw).
  - One VRC6 step = `pulse_out(15, 0) / 15`, from the VRC6 audio page: "at maximum volume, the
    pulse channels of the VRC6 are roughly equivalent to the pulse channels of the 2A03 (except
    inverted)".
  - Mixed with inverted polarity.
  - **Sources checked for the VRC6 level:** VRC6 audio (2018 revision), VRC6, NSF, Namco 163 audio
    (it has dB tables, but only for the N163), Talk:VRC6 audio (2011: an unanswered "how loud?"
    question). The Expansion audio page returns 404.
  - **No source disagrees, because only one makes any level claim, and it is qualitative.** That
    makes it the most uncertain number in the engine (§6.5, gap 1).

### 3.4 Output stage (switchable: F7, `CARTWAVE_FILTER`)

| Mode | Stages | Source |
|---|---|---|
| **NES (default)** | HP 90 Hz → LP 14 kHz (first order) | APU Mixer, current revision (27 Oct 2026) |
| NES + 440 | HP 90 → HP 440 → LP 14 kHz | APU Mixer revisions 2014-09-16 to 2026-10-27 |
| Famicom | HP 37 Hz | APU Mixer (since 2019; was 8 Hz before) |
| Off | 5 Hz DC blocker only | — (speaker safety) |

Each stage is a bilinear first-order section, pre-warped so the −3 dB point is exact at 48 kHz.

**The disagreement, and how it was resolved with a third source.**

| Source | What it says |
|---|---|
| Wiki, 2014–2026 | Three stages: 90 Hz HP, 440 Hz HP, 14 kHz LP |
| Wiki, current | Two stages. The edit summary says the 440 Hz filter "is wrong… Enough people have implemented it and observed it sounds wrong". No measurement is cited |
| blargg's 2009 forum post (the primary source) | The three stages are fitted coefficients from recordings, made through a PC sound card |
| lidnariq's reply | Matches 90 Hz to 150 Ω × 10 µF and 14 kHz to 47 kΩ × 220 pF, but says of 442 Hz: "I haven't yet found what it corresponds to" |

The third source is the schematic's component values:
- 150 Ω × 10 µF gives 106 Hz, and 47 kΩ × 220 pF gives 15.4 kHz.
- Nothing in the circuit produces ~440 Hz.
- Also, blargg's own fitted coefficients, converted at the stated 44.1 kHz, give 1.2 Hz and 28 Hz,
  not 90 and 442 Hz. The derivation that produced the 440 Hz figure is internally inconsistent.

**Decision:** the two-stage filter by default, with the three-stage one kept for A/B. **Famicom:**
the RF modulator path is not modelled.

## 4. The sound driver (`chip::driver`, `chip::player`)

- **Frame ticks.**
  - The driver runs every 29 780.5 CPU cycles (60.0988 Hz), scheduled at the exact cycle inside the
    render loop, the way a game's NMI music routine runs.
  - Measured on the live engine: 7214 ticks in 120.0 s = **60.105 Hz**. The 0.01% excess is from
    counting a tick at t = 0.
  - The APU frame counter runs on its own 29 830-cycle sequence, so it drifts against the driver,
    as on hardware.
- **Tempo.**
  - The song position is continuous in emulated time: ticks/s = BPM / 60 × 960.
  - At each frame tick the driver plays the note under the position, so **note starts and ends are
    quantised to frame ticks (≤ 16.6 ms late)**. At 150 BPM one frame is 39.9 ticks and a 1/16 note
    is 6.01 frames.
  - A note shorter than a frame still sounds for one frame (like a tracker row). After a seek, only
    notes still running are chased.
- **Note-on / off per channel** (only what the hardware has):

| Channel | Note-on | Sustain (parameter change) | Note-off |
|---|---|---|---|
| Pulse | $4000 (duty, L, C, V), $4001 (sweep, or $08 = off with negate so low notes aren't muted), $4002, $4003 (restarts the envelope, resets the phase) | $4000 / $4001 only: no phase reset | $4000 ← constant volume 0 |
| Triangle | $4008 ← $FF (control + reload 127), $400A, $400B | — | $4008 ← $80 (linear counter reloads 0 at the next quarter frame: up to 4.2 ms) |
| Noise | $400C, $400E (mode + period index from the note or fixed), $400F | $400C | $400C ← 0 |
| DMC | $4010 (loop, rate), $4011 ← 64, $4012, $4013, $4015 D off then on (restart) | $4010 | one-shots run out; a looping sample is stopped ($4015) |
| VRC6 pulse | $9000/$A000 (mode, duty, volume), $x001, $x002 (enable) | $x000 | volume 0 |
| Saw | $B000 (rate), $B001, $B002 (enable) | $B000 | clear E (accumulator held at 0) |

- **Pitch:** nearest period per oscillator.
  - The tuning error for every MIDI note 21–108 is in [`docs/tuning.md`](docs/tuning.md).
  - Worst error in C2–C7: pulse +14.5 ¢ (C7), triangle +19.2 ¢ (B6), VRC6 pulse +14.5 ¢, saw −11.1 ¢.
    The error grows with pitch because periods get short.
  - The 2A03 pulse can't go below A1 (t ≤ $7FF). Out-of-range notes are skipped, not wrapped.
- **Noise:** "period" is `(96 − note) / 4`, or a fixed index from the panel.
- **DPCM:** C = kick, C♯ = snare, D = tom, D♯ = crash (repeating), or a fixed sample. The four
  samples are synthesised by `chip::samples` and encoded against the real 7-bit counter.

## 5. Real-time host and UI wiring

- **Commands:** song snapshots, play/stop/seek, BPM, loop end, mute/solo masks, per-lane
  parameters, VRC6 on/off, filter, gain. Test-only commands: stall, pilot tone, click.
- **Snapshots.**
  - The UI hashes the audible content each pump tick. On a change it builds a `SongSnapshot` and
    sends it. That is at most one per pump tick (4 ms while active), so up to 250 a second; test B
    sent ~200 a second.
  - Building one for 200 clips takes ~0.1 ms on the UI thread.
  - Measured send → engine latency was ≤ 10.8 ms (one device period); dequeue → frame tick ≤ 16.64 ms.
- **Playhead (the audio clock is the master).**
  - Each callback publishes, under a seqlock: the song position of its first frame, ticks per
    frame, the callback time, and the latency (cpal's `playback − callback` plus the 0.33 ms
    band-limiting delay).
  - The UI computes the position that will be audible **when its next frame is presented** (last
    present + present interval, from PERF.md's frame timings). It does not use "now".
  - This one change moved the A/V offset from −27.6 ms to −11.0 ms (§7, G).
  - While a command is in flight, or with no device, the UI keeps integrating its own position.
- **Meters / lights.**
  - Per-channel peak DAC levels are max-merged by the audio thread and taken by the UI each frame.
    The DMC meter shows its swing, because its level is a DC offset.
  - Channel lights fire when a note-on becomes *audible* (note-on time + latency), not when it is
    queued.
- **Monophonic overlap rule** (`Arrangement::resolve_overlaps`, unit-tested):
  - The clip being dropped, moved, resized or duplicated wins. Any clip it overlaps in the same lane
    is trimmed at the end, trimmed at the start (notes keep their song positions), split in two (the
    right part becomes a new clip), or removed.
  - During a drag the model's other clips are left alone, but the engine's snapshot already applies
    the rule with the dragged clips winning. On drop it is applied to the model.
  - `SongSnapshot::build` also trims any remaining note overlaps, as a backstop. **The engine never
    receives overlapping notes on one channel.**
- **Per-channel panels:** the device panel rebuilds for the selected clip's lane and shows only the
  hardware controls:

| Lane | Controls |
|---|---|
| Pulse 1/2 | Duty (12.5/25/50/75%), Volume (V), Envelope (const / decay / loop), Sweep (−7..7: up/down shift), Sweep rate (P) |
| Triangle | none ("the triangle has no volume or duty") |
| Noise | Mode (long/short), Period (from note / 0–15), Volume, Envelope |
| DPCM | Sample (from note / Kick, Snare, Tom, Crash), Rate (0–15), Loop |
| VRC6 pulse | Duty (1/16 … 8/16, digitized), Volume |
| VRC6 saw | Rate (0–63; > 42 wraps) |

- **Device errors:** the transport bar shows "AUDIO: device changed, reconnecting", "AUDIO: no
  output device", or "AUDIO: can't open output". The F3 overlay shows the device, frames per
  callback, latency, callback cost and underruns.

## 6. Authenticity verification

No hardware was available. What each check can and can't prove:

| Check | Proves | Doesn't prove |
|---|---|---|
| 6.1 Unit tests (58 in `chip`) | The implementation matches the *documented* behaviour: frequencies from periods (all oscillators), duty waveforms, triangle sequence, LFSR loop lengths and first outputs, envelope timing, sweep muting thresholds, length table, DMC rate table and delta rules, VRC6 saw steps, frame counter cycles | That the documentation is right or complete |
| 6.2 Mixer tests | The formulas are implemented exactly; the documented tables are within 0.4% / 4.66% of them | That the formulas match a real console (the wiki itself calls them an approximation) |
| 6.3 Spectral checks | Band-limiting removes aliasing (−85…−89 dB) without changing pitch (≤ 0.1 ¢) | Anything about the analog stage beyond the documented corners |
| 6.4 Listening pack | Gives the team material to compare against hardware recordings | Nothing on its own |
| Event-driven = reference | The optimisations change nothing | — |

### 6.5 Known gaps, ranked by how audible they're likely to be

1. **VRC6 level and polarity relative to the 2A03.**
   - The only source is qualitative ("roughly equivalent"). Real VRC6 carts mix through
     board-specific resistors into the Famicom's expansion audio pin, so the level varies between
     carts.
   - A 2–3 dB error would audibly change the balance of every VRC6 song.
   - *Next step:* measure from a hardware recording of a VRC6 cart, then make the level a calibrated
     constant.
2. **The analog output stage.**
   - The 440 Hz stage is disputed (§3.4). The corners come from a 2009 curve fit plus component
     matching. Consoles differ (front-loader vs top-loader vs Famicom, RF vs AV, mods), and the
     Famicom RF path is unmodelled.
   - Audible as bass weight and treble.
3. **Non-linear mixer accuracy.** The wiki's own history notes the formula "is also an
   approximation, as MDFourier reveals". This affects how loud channels are relative to each other
   in dense passages.
4. **VRC6 details marked uncertain in the source:**
   - "it seems that the accumulator only reacts on every 2 clocks" (saw pitch and timbre);
   - the 16×/256× scaling is described only in prose;
   - "the DAC… appears to be linear".
5. **Register-write timing inside a frame.** All of one tick's writes land on the same CPU cycle; a
   real driver spreads them over a few hundred cycles. This shifts phase relationships by
   microseconds, which is inaudible except, perhaps, the DMC restart.
6. **Driver behaviour** (a design choice, not emulation):
   - $4011 ← 64 on every DPCM hit (a tiny click);
   - pulse phase reset on every note-on;
   - note-offs by volume 0.
   Real game drivers vary, so this matters when matching a specific game's sound.
7. **Not modelled:**
   - DMC DMA stalls and their CPU-side effects (no CPU here);
   - frame IRQ timing (set, never read);
   - same-cycle race conditions (e.g. a length load on the clocking cycle);
   - early 2A03 revisions;
   - PAL.
   None of these is audible in this use.
8. **Band-limiting** attenuates content above ~19 kHz (cutoff 21.6 kHz). This is inaudible, and the
   console's own 14 kHz low-pass is far below it.

## 7. Stress tests (brief §5)

All runs: release build, the MOTU M Series as default output in WASAPI shared mode, 48 kHz, gain
0.3, playing, with a 19 kHz pilot tone mixed in. A second process captures the output through
WASAPI loopback and counts **dropouts** (pilot missing for a block). That is the ground truth;
the engine's own detectors ("starved", "late"; §8.1) are reported beside it. Driver:
`docs/engine/tools/matrix.ps1` (about 80 minutes); table: `summarize.py`; raw per-second logs:
[`docs/engine/results/`](docs/engine/results/).

**The requested buffer size is nominal on Windows.** WASAPI shared mode ignored it: every run got
512-frame callbacks (10.67 ms), plus one 1126-frame first callback per stream (§8.1). The three
sizes were run anyway, as the brief asks. They show that nothing depends on the request.

| Test | Requested | Run | Frames / callback | Underruns: **loopback dropouts** / starved / late | Callback cost, % of period: mean / p99 / max | Audio thread, k-cycles / callback | UI thread, k-cycles / frame | Other |
|---|---|---|---|---|---|---|---|---|
| A. Demo set playing | 128 | 600 s | 512 (first 1126) | **0** / 0 / 0 | 0.22 / 0.6 / 3.3 | 148 | 4498 | |
| | 256 | 600 s | 512 | **0** / 0 / 0 | 0.23 / 0.6 / 3.5 | 149 | 4461 | |
| | 512 | 600 s | 512 | **0** / 0 / 0 | 0.22 / 0.6 / 3.4 | 145 | 4440 | |
| B. 200 clips + scripted edits, every pump tick | 128 | 600 s | 512 | **0** / 0 / 0 | 0.08 / 0.3 / 2.9 | 87 | 12110 | 149,995 edits (250/s); 118,239 snapshots applied (197/s); send → engine ≤ 10.8 ms; dequeue → frame tick ≤ 16.64 ms; 0 commands dropped |
| | 256 | 600 s | 512 | **0** / 0 / 0 | 0.08 / 0.3 / 2.9 | 86 | 12105 | same |
| | 512 | 600 s | 512 | **0** / 0 / 0 | 0.08 / 0.3 / 3.1 | 86 | 12099 | same |
| C. UI thread spins 250 ms every 2 s | 128 | 120 s | 512 | **0** / 0 / 0 | 0.22 / 0.6 / 3.1 | 143 | 15585 | 60 stalls (the spin is counted in the UI cycles) |
| | 256 | 120 s | 512 | **0** / 0 / 0 | 0.22 / 0.6 / 3.5 | 143 | 15555 | 60 stalls |
| | 512 | 120 s | 512 | **0** / 0 / 0 | 0.22 / 0.6 / 3.2 | 143 | 15459 | 60 stalls |
| D. Resize sweeps, minimise, restore | 128 | 120 s | 512 | **0** / 0 / 0 | 0.16 / 0.3 / 3.0 | 107 | 2510 | A/V −10.95 ms, sd 0.23 |
| | 256 | 120 s | 512 | **0** / 0 / 0 | 0.16 / 0.3 / 2.9 | 107 | 2511 | A/V −10.99 ms, sd 0.23 |
| | 512 | 120 s | 512 | **0** / 0 / 0 | 0.16 / 0.3 / 2.9 | 108 | 2513 | A/V −10.97 ms, sd 0.45 (range −12.16 … −8.74) |
| E. Default output switched to the monitor (HDMI) and back | default | 50 s | 480 on HDMI, 512 on MOTU | see below / 2 / 2 | 0.22 / 0.5 / 1.2 | 143 | 4507 | 2 rebuilds, no crash, song and position kept |
| F. Callback cost | all | | | | p99 ≤ **0.6%**, max ≤ **3.5%** in every run (target: p99 < 10%) | | | |
| G. A/V sync (1-bar loop, a click at mid-bar) | 128 | 120 s | 512 | **0** / 0 / 0 | 0.16 / 0.3 / 3.2 | 108 | 1827 | 75/75 pairs: −10.98 ms, sd 0.11, range −11.28 … −10.72 |
| | 256 | 120 s | 512 | **0** / 0 / 0 | 0.16 / 0.3 / 3.1 | 108 | 1804 | −10.97 ms, sd 0.10 |
| | 512 | 120 s | 512 | **0** / 0 / 0 | 0.16 / 0.3 / 3.2 | 109 | 1814 | −10.97 ms, sd 0.10 |

Zero allocations on the audio thread in every run (`NoAllocGuard` + the checking global allocator,
which counts any allocation, free or reallocation made inside the callback).

Notes:

- **G (A/V sync): within ±1 frame, with margin.** The offset is *audio − picture*: the click reaches
  the loopback 11.0 ms *before* the picture shows the playhead crossing it. Its spread is 0.1 ms.
  The 11.0 ms equals the device stream latency (10.67 ms) plus the band-limiting delay (0.33 ms).
  That is, loopback taps the signal before the device's own latency, so a listener at the
  speakers hears the click within about 0.3 ms of the picture. This also checks that the
  latency compensation is right: before the playhead was predicted for the present time the offset
  was −27.6 ms (§5).
- **D:** the resize sweeps and minimise / restore never disturbed the audio. The A/V spread widens
  (sd 0.23–0.45 ms) because frames are dropped or late while the window changes size, which is
  expected. *Not tested:* moving the window between displays with different refresh rates. This
  machine has one monitor.
- **E:** each switch leaves a **~0.4 s gap** (max callback gap 400 ms): the old stream is
  invalidated, the host waits 250 ms for Windows to settle the new default, and rebuilds. The
  engine core, song and position carry over. The 2 "starved" / 2 "late" counts are those two
  gaps, so they are correct detections of a switch, not glitches. The loopback can't judge this
  test: it captures from the default device, so it was interrupted by the same switches
  (5,936 blocks instead of ~18,750). It reported no dropouts during the blocks it did capture.
  Physical unplugging was not tested (§9).
- **F:** the cost is the time inside `Core::process` as a share of the callback period, in a
  0.1%-bucket histogram. Cost per callback scales with its length, so the percentage hardly depends
  on the buffer size. The rare 3% maxima (0.3 ms, against a 0.2% mean) look like preemption
  rather than work. On Windows the audio thread runs at normal priority (§8.1); this was not
  traced.
- **Cycles, instructions, energy.** The cycle columns are per-thread TSC cycles
  (`QueryThreadCycleTime`).
  - Audio cycles include cpal and WASAPI's work around our callback, so they read higher than the
    callback cost alone.
  - B plays the 200-clip stress set, not the demo song. Its audio cost differs for that reason
    alone; the chip's cost depends on what is playing.
  - The UI cost follows what is drawn and edited. G draws a short loop with the music muted. A is
    the demo. B builds a 200-clip snapshot up to 250 times a second. C includes the injected spins.
  - **Instructions retired and energy were not measured on Windows.** Per-thread instructions need
    the hardware PMC counters through an elevated ETW session. Energy is not exposed for this CPU
    through any interface available here. Both are available on macOS (PERF-macos.md) and should
    be added in the Mac pass.

**The UI budget with the engine running** (PERF.md's own scripts, `docs/perf/tools/bench.ps1` and
`pacing.ps1`, process CPU in % of one core, 3 runs). This includes the audio thread:

| Scenario | PERF.md, before the engine | With the engine | Re-measured, engine verified playing (PIANOROLL.md Part 0) | Pacing, 30 s, with the engine |
|---|---|---|---|---|
| Idle | 0.0–0.1% | **0.0–0.4%** | 0.1 / 0.4 / 0.7% | no frames |
| Demo playing | 9.1 / 9.4 / 9.6% | **9.6 / 9.8 / 10.3%** | 8.9 / 9.1 / 10.0% | 1800 frames, p99 16.90 ms, 0 > 20 ms, 0 missed vblanks |
| 200 clips playing | 9.1 / 9.8 / 9.9% | **10.1 / 10.5 / 10.8%** | 9.2 / 9.5 / 9.8% | p99 16.88 ms, 0 > 20 ms, 0 missed |
| 200 clips, dragging | 10.0 / 10.9 / 11.2% | **10.6 / 10.7 / 12.1%** | 10.2 / 10.8 / 10.8% | p99 16.91 ms, one 20.7 ms submit interval, 0 missed vblanks |

**Correction note (PIANOROLL.md Part 0, 2026-11-24).** ENGINE-macos.md's bug 3 (F4 lit "playing"
without starting the engine, fixed in `fbc2309`) suggested that the "200 clips playing" row above
was measured with the engine stopped. **On Windows it was not.** `bench.ps1` presses Space (which
sends `Play`) *before* F4, so the engine was already playing when F4 loaded the 200 clips; only
sequences that start with F4 alone (the macOS `chipbench` "200" scenario) hit the bug. The
deciding probe: the pre-fix build (`4027a25`) run through `bench.ps1`'s exact key sequence with
`CARTWAVE_AUDIOLOG` logged **0 playhead frames per second on the local clock** throughout the
200-clip phase, i.e. every frame's playhead came from the playing engine. The original numbers
therefore stand. The re-measured column (current build, fork commit 10, the F3 overlay showing
"clock audio 60/s local 0/s") agrees within run-to-run noise. Two differences from the original
runs: the default output device is now the monitor's HDMI endpoint (480-frame callbacks), not the
MOTU; and during drags 4–21 frames per second take the local clock because a snapshot command is
in flight (`audible_position_at` returns `None` while `pending() > 0`, by design, §5).

All of PERF.md's targets still hold. The engine adds roughly 0.5 points of one core, which is the
audio thread. The first measurement found **idle at 2.1–2.5%**. The cause: the hidden F3 overlay
was being sent fresh audio statistics every 250 ms, which drew a frame and kept the 4 ms pump fast,
so the app never went idle. The statistics are now published only while the overlay is visible.

## 7a. The reference match: the audio gate since Batch 4c

**Why.** The pilot tone (§7) can be masked: at full Main volume, 1,024 band-limited studio voices
put energy into its band and it reported ~6 false dropouts a second (BATCH4B.md §4). Batch 4c
replaced it as the gate's ground truth (pre-approved, BATCH4C_PROMPT.md §0.2).

**How.** WASAPI shared-mode loopback returns exactly the samples we hand the device
(`crates/engine/examples/tapprobe.rs`: 187,200 of 187,200 frames bit-identical, gain 1.000000).
So instead of listening for a tone, the harness compares the capture with **the engine's own
copy of its output**:
- `Shared::tap` (`testkit::OutputTap`, set once by the harness): the audio thread stores the first
  output channel, as written to the device, into a preallocated ring of atomics (no allocation,
  no locks; tested under the allocation checker).
- A checker thread (`testkit::Matcher`) locks onto the copy in the capture with 1,024 equal
  samples, then compares every 128-frame block exactly (tolerance 1e-5). A **dropout** is silence
  the device inserted: the capture's offset from the copy jumps forward by the gap's length, in
  frames (a sample counter, in effect). Backward jumps are capture-side gaps; a mismatch that
  re-locks at the same offset is foreign sound.
- A −60 dBFS pseudo-random **marker** (`Command::TestMarker`) keeps the alignment unique through
  silence and held notes. Silence never locks. The music can't mask anything: it is subtracted
  exactly.

Hooks: `CARTWAVE_MATCH=1` (app; the audio log gains `match_dropouts`, `match_mismatch_runs`,
`match_blocks`, `match_samples` and a `# match:` summary), `--match` (headless);
`CARTWAVE_INJECT_STALLS=ms,…` / `CARTWAVE_INJECT_EVERY=s` inject audio-callback stalls in the app.

**Validated** (`docs/batch4c/audio/`): 23 stall lengths 5…50 ms, 3 passes, music muted and
playing, on the HDMI endpoint (480-frame periods): **the match, the pilot (muted: it can't be
masked) and the starved detector agree on all 138 stalls**: stalls ≤ 19 ms never drop out, ≥ 21 ms
always do (20 ms: 5 of 6, the same ones in all three). The reported gaps are multiples of the
device period (8, 18, 28, 38 ms). In the app at full volume on the 1,024-voice project, with
stalls of 10/19/30/40 ms: the match finds exactly the 12 stalls of 30/40 ms and none of the 12
shorter ones; the pilot reports 460 "dropouts". The ≥ 22 ms threshold of §8.1 is the MOTU's
(a 1,126-frame buffer); each device has its own.

## 7b. The performance gate: cycle counters and a noise allowance (since the Presets + Browser batch)

The rule every later prompt inherits (PRESETS_PROMPT.md §2, from BATCH4C.md §2's findings):

- **The primary measure is per-thread cycle counters** (Batch 4c's method,
  `docs/batch4c/tools/threads_b4c.ps1`): **UI cycles per presented frame** and **audio cycles per
  callback**, read from the threads' `QueryThreadCycleTime`. Process CPU % (tick-sampled, its
  audio share swung 3.4–6.4 % between runs of one build) is still reported, for context.
- **10 interleaved rounds** of the two builds, the order alternating each round; judged on the
  **means**, with the **standard error of the paired round differences** reported (the noise
  floor).
- **A scenario fails only if its difference is over its limit *and* more than 2× its noise
  floor.** A difference inside 2 SE of zero is not evidence of a regression and is reported as
  noise, never re-run to fish for a pass.
- Every run proves it tested what it claims (the scenario's proof line, focus held), as before.

## 8. Windows audio findings, and what will differ on macOS

### 8.1 Windows (WASAPI shared mode through cpal 0.18.2; probes in `crates/engine/examples/probe.rs`, `headless.rs`)

| Finding | Evidence |
|---|---|
| **The requested buffer size has no effect on the callback size.** 128, 256, 512 and "default" all give 512-frame callbacks every 10.67 ms on the MOTU, and 480 frames (10 ms) on the NVIDIA HDMI endpoint | cpal's source: shared-mode `Initialize`, "the callback period is always GetDevicePeriod() regardless of what is requested here". The probe and every test log agree |
| Device buffer = 1126 frames (23.5 ms; the first callback of a stream gets all of it). Playback lead (cpal `playback − callback`) = **34.1 ms** steady; the device's own stream latency is **10.67 ms** | probe; `ahead` in the logs. The lead collapses to exactly 10.67 ms when the buffer runs dry |
| **After one underrun, WASAPI keeps a lower lead for the rest of the stream** (34.1 → 12.8 ms) | probe sweep. This is why latency compensation is re-read on every callback rather than measured once |
| The glitch signature: a callback asking for **more than one period** (614 instead of 512 frames) | Stall sweep 16–30 ms against loopback: stalls ≤ 21 ms were clean and asked for 512 frames; every stall ≥ 22 ms dropped out and asked for 614. This "starved" detector agreed with loopback 4/4 in a mixed run, with no false positives. The obvious alternatives were wrong: "gap > buffer" missed 22–23 ms stalls, and "callback was handed the whole buffer" never fired |
| Lower latency is not reachable through cpal | It uses neither `IAudioClient3::InitializeSharedAudioStream` (small shared-mode periods) nor exclusive mode. ASIO is excluded by licence |
| Default-device switch: cpal's default-device monitor reports `StreamInvalidated`. The host rebuilds on the new default in ~0.3 s and back again (test E) | test E log: generation 1 → 2 (NVIDIA, 480 frames) → 3 (MOTU) |
| WASAPI loopback capture reports one "buffer underrun or overrun" at start | Capture-side startup notice; it appears even in clean runs (0 dropouts) |
| Audio-thread priority | cpal (default features) does not raise it; WASAPI's event thread runs at normal priority. Even so, 0 underruns, with the UI pinning a core in 250 ms bursts. The `realtime` feature (MMCSS) would add `audio_thread_priority` (MPL-2.0), so it was left off |
| Dependency licences | cpal (Apache-2.0), rtrb, dasp_sample, windows-* 0.62 (MIT/Apache), unicode-ident (MIT/Apache + Unicode-3.0). No GPL/LGPL/MPL added by the engine |

### 8.2 Likely to differ on macOS / CoreAudio (from cpal's CoreAudio source; to be verified in the Mac pass)

- **Buffer sizes.**
  - cpal sets `kAudioDevicePropertyBufferFrameSize`, so 128 / 256 / 512 should be granted as
    requested. The test matrix becomes meaningful there.
  - Callback cost as % of the period will be 4× higher at 128 frames. That is still far under 10%
    at the costs measured here.
- **Default-device change.**
  - The AudioUnit reroutes by itself and cpal reports `DeviceChanged`. The host ignores that error
    rather than rebuilding (fixed in source before the Mac pass).
  - A removed device with no replacement reports `DeviceNotAvailable`; the host then waits and
    retries.
- **The audio thread is CoreAudio's real-time IO thread** (time-constraint priority). That is better
  than Windows' default.
- **Underrun detection.**
  - The "starved" signature is WASAPI-specific. CoreAudio calls back with a fixed size and reports
    overloads differently, so on the Mac the "late" detector and loopback ground truth carry the
    weight.
  - Loopback: cpal builds an aggregate device with a process tap (`macos/loopback.rs`). This needs
    a recent macOS and the audio-capture permission, and must be tried before relying on it.
- **Instrumentation.**
  - `engine::cpu` per-thread cycles are Windows-only, so they read 0 on the Mac. There,
    `proc_pid_rusage` gives instructions and energy (PERF-macos.md), and per-thread cycles can come
    from `thread_info`.
  - `set_default_audio.ps1` and `matrix.ps1` are Windows-only. The Mac needs an equivalent
    (`SwitchAudioSource` is GPL, so use a small CoreAudio Swift tool instead).
- **Timestamps.** cpal derives CoreAudio's `playback` from the host time plus device and stream
  latency. The A/V test (G) should be repeated there, since its zero point is different.
- Nothing in `chip`, `engine::core`, the queues or the UI wiring is platform-specific.

## 9. Verdict

**Rust + vizia + a real-time chip engine is viable for the product, on the evidence of this
machine.**

- **Glitch-free under load.** 0 loopback dropouts across 15 runs and 3.5 hours of playback:
  - a 250 ms UI spin every 2 s;
  - ~200 new 200-clip song snapshots per second for 10 minutes;
  - window churn.

  The callback uses under 1% of its period at p99. The design keeps the UI from ever reaching the
  audio thread: a wait-free queue, immutable snapshots, retired memory freed on the UI thread, and
  no allocation in the callback (enforced and measured).
- **Responsive to edits.** An edit reaches the engine within one device period (≤ 10.8 ms). It is
  heard at the next frame tick (≤ 16.64 ms), which is the console's own timing resolution.
- **In sync.** The picture is within 0.3 ms of what is heard at the device (spread 0.1 ms). The
  playhead follows the audio clock.
- **Authentic, as far as can be checked without hardware.**
  - Every channel matches the documented behaviour (58 tests).
  - The mixer uses the documented formulas.
  - Aliasing is 85 dB down.
  - The biggest open question is the VRC6 level against the 2A03 (§6.5). It needs one hardware
    recording.
- **The UI budget is unchanged.** PERF.md's targets still hold with the engine running.

Conditions and limits:

1. **Latency on Windows is ~34 ms** (WASAPI shared mode through cpal, a 10.67 ms period). That is
   fine for arranging and playback. For live keyboard input, it would need a backend outside cpal's
   current Windows support (`IAudioClient3` small shared-mode periods, or exclusive mode). This is
   a project decision, not a blocker.
2. **On Windows the audio thread runs at normal priority.** This was not a problem here. It might
   be on a slower or busier machine. MMCSS registration (cpal's `realtime` feature) pulls in an
   MPL-2.0 crate, so it needs a licence decision, or a few lines of our own FFI.
3. **Device changes cost a ~0.4 s gap and recover by themselves.** Physically unplugging the
   interface, sleep / resume, and moving the window between displays were not tested.
4. **One machine.** All of this is a fast desktop CPU. The Mac pass (§8.2) should repeat the matrix
   with real 128 / 256 / 512 buffers, which Windows did not grant, and measure instructions and
   energy.
