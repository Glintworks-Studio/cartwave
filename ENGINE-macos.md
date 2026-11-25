# ENGINE-macos: the real-time chip engine on macOS (Apple M1, MOTU M2)

Companion to [ENGINE.md](ENGINE.md) (Windows) and [PERF-macos.md](PERF-macos.md). This pass
**verifies** the engine on macOS. It adds no features.

> **Status: stopped early, at the user's direction.** During test J the user saw content flashing
> on the 60 Hz display. The stress matrix (§4) was stopped so that could be fixed first; it was
> (§7, bug 5). The matrix has **not** been run in full on the final build: only G at 64 and 128
> frames completed.

**Summary so far**

- **Five real bugs found and fixed**, each with before/after numbers (§7):
  1. A CoreAudio overload rebuilt the whole stream: **~350 ms of silence per overload** → now
     counted, 0 rebuilds, 17 ms worst gap.
  2. The "late" detector fired on normal CoreAudio jitter: **~95 false "late"/s** → 0 in clean runs.
  3. F4 (200-clip set) showed "playing" while the engine stayed stopped (also on Windows).
  4. A device change (unplug, default switch) left the stream on the wrong buffer size with blind
     detectors and no message → rebuild on the new device, "AUDIO: now playing on …".
  5. **The window flashed on a second display**: after a move to the 60 Hz display the whole window
     flipped between two colour conversions, 38–67 times per 20 s (975 uncapped) → **0** (vizia fork
     commit 8 pins the window to sRGB; confirmed by the user).
- **No valid ground truth for our own dropouts on macOS** without a physical loopback cable
  (§3): cpal's loopback records the M2's inputs, and a CoreAudio process tap is blind to the
  tapped process's lateness. Dropouts here are CoreAudio's overload reports.
- **CoreAudio grants every requested buffer size exactly** (64 / 128 / 256 / 512), unlike WASAPI
  shared mode. The audio thread is CoreAudio's real-time IO thread (round-robin, priority 97,
  time-constraint = one period).
- **The UI budget is unchanged by the engine** once bug 3 is fixed: +2–6% instructions, the same
  vblank cadence as PERF-macos.md.

---

## 1. The two agreed changes

| Change | Done | Evidence |
|---|---|---|
| **Filter default follows the chip.** VRC6 on → Famicom (HP 37 Hz); 2A03 alone → NES (HP 90 Hz, LP 14 kHz). A filter picked with F7 (or `CHIPDAW_FILTER`) is kept from then on. The F3 overlay shows `filter FC (auto)` / `filter NES (set)` | `ac6b3fd` | `FilterMode::default_for` unit test; the app starts on 2A03 + VRC6, so the default is now Famicom |
| **Frame-rate cap default = 60 fps**, F6 cycles 60 → 120 → off, "display rate" removed (`CHIPDAW_FPS_CAP=display` is an alias for off) | `dd94e56` | `frame_cap_default_and_aliases` unit test; every §5 run below presents exactly 60.0 fps with no cap variable set |

**Demo renders** (`674660c`), same offline `Player` path as the existing pack
(`cargo run --release -p app -- --render-demo docs/audio 25.6`):

| File | Peak |
|---|---|
| `docs/audio/demo_excerpt_nes.wav` | 0.563 |
| `docs/audio/demo_excerpt_famicom.wav` (new) | 0.629 |
| `docs/audio/demo_excerpt_raw.wav` | 0.736 |

The NES and raw files rendered on the Mac are **byte-identical** to the ones committed from
Windows (same SHA-1). The emulator and the band-limiting are deterministic across x86-64 and
arm64.

## 2. Verified environment

| Fact | Value | Sources |
|---|---|---|
| macOS | 26.6.2 (25G83), Darwin 25.6.0 | `sw_vers`; `uname -a` |
| Machine | Mac mini (Macmini9,1), Apple M1, 4 P + 4 E cores | `sysctl hw.model`, `hw.perflevel0/1.logicalcpu`; `hw.perflevel0.name` = Performance |
| Rust | rustc 1.100.0 (0ef1bc386 2026-11-09), aarch64-apple-darwin | `rustc -Vv` |
| App commit | pass started at `1232bfe` (Windows ENGINE.md); fixes on top, see §7 | `git log` |
| vizia fork | `40ef8cfe` (unchanged; resolves from GitHub) | `Cargo.toml` pin; `Cargo.lock` source line |
| cpal | 0.18.2, CoreAudio host, default features | `Cargo.lock`; probe prints `host CoreAudio` |

### 2.1 Audio devices

Two sources agree on every field below: `system_profiler SPAudioDataType` and our CoreAudio tool
(`coreaudio list`, `docs/engine/tools/macos/coreaudio.swift`).

| Device | Transport | Nominal / available rates (Hz) | Buffer frames: current, range | Output latency + safety offset |
|---|---|---|---|---|
| **MOTU M2** (default output, chosen for the matrix) | USB | **48000** / 44100–192000 | 512, 15–4096 | 14 + 14 frames (0.29 + 0.29 ms) |
| Mac mini Speakers (system sounds) | built-in | 48000 / 44100–96000 | 512, 15–4096 | 64 + 116 frames, stream latency 408 |
| GA27T1M (320 Hz display) | DisplayPort | 48000 / 32000–48000 | 512, 15–4096 | 88 + 320 frames |
| XB271HU (60 Hz display) | HDMI | 48000 / 32000–48000 | 512, 15–4096 | 96 + 576 frames |
| Sound Blaster GS3 | Bluetooth | 44100 only | 512, 14–1024 | 9900 frames (224 ms) |

The M2's native rate is 48 kHz, so only 48 kHz was tested: no resampling anywhere. Note that cpal
sets the device's hardware sample rate to the requested one when it opens a stream
(`set_physical_format`, device.rs). On a device running at 44.1 kHz for another app, opening the
engine at 48 kHz would switch it for everyone.

### 2.2 What cpal actually grants (`cargo run --release -p engine --example probe_mac`)

| Requested | `stream.buffer_size()` | Frames per callback | Callback gap ms (min / mean / max) | cpal `playback − callback` | cpal `callback` − now |
|---|---|---|---|---|---|
| 64 | 64 | **64 every time** | 1.300 / 1.333 / 1.362 | 1.917 ms | +1.63 ms |
| 128 | 128 | **128** | 2.618 / 2.667 / 2.718 | 3.250 ms | +2.96 ms |
| 256 | 256 | **256** | 5.299 / 5.333 / 5.373 | 5.917 ms | +5.63 ms |
| 512 | 512 | **512** | 10.602 / 10.667 / 10.759 | 11.250 ms | +10.92 ms |
| default | 512 | 512 | 10.604 / 10.667 / 10.754 | 11.250 ms | +10.92 ms |

- **CoreAudio honours the request exactly**, as ENGINE.md §8.2 predicted, and the callback size
  never varies. WASAPI shared mode gave 512 regardless.
- **The buffer size is per process.** With our stream at 128 frames, another process
  (`coreaudio list`) still read 512 on the M2. That's three sources: the callback sizes,
  `stream.buffer_size()` in process, and the device property read from outside.
- **Output latency = buffer + 0.58 ms** (buffer + 14 frames device latency + 14 frames safety
  offset). At 64 frames that is 1.9 ms; at 128, 3.3 ms; at 256, 5.9 ms; at 512, 11.3 ms. Windows:
  ~34 ms.
- **cpal's `callback` timestamp is not the callback time.** It is CoreAudio's output time stamp,
  one period plus the safety offset (0.29 ms) in the future. cpal then adds the buffer again to
  get `playback`, so cpal's absolute `playback` instant is one buffer too late. The engine only
  uses the *difference* `playback − callback` measured from its own "now", which comes out at
  buffer + latency + safety offset: the right lead by construction. The end-to-end check (G) couldn't be validated without a physical loopback (§4).
  Possible upstream issue (§9).

### 2.3 The audio thread

| | Value | Sources |
|---|---|---|
| Thread | `com.apple.audio.IOThread.client`: CoreAudio's IO thread, not one of ours or cpal's | `proc_pidinfo(PROC_PIDTHREADID64INFO)` name |
| Scheduling | policy 2 (**round-robin**), priority **97** | same call (`pth_policy`, `pth_priority`) |
| Time constraint | period = computation = constraint = **one buffer period** (1.333 / 2.667 / 5.333 / 10.667 ms), preemptible | `thread_policy_get(THREAD_TIME_CONSTRAINT_POLICY)` on the same thread |
| Cores | **P-cores at 64–256 frames, E-cores at 512** (P-core time 0.0 ms of 6.6 ms in 3 s) | `PROC_PIDTHREADCOUNTS` per-perf-level time |

So the Mac audio thread is real-time, unlike Windows' normal-priority WASAPI thread (ENGINE.md §8.1).
At 512 frames the scheduler parks it on the efficiency cores: callbacks take ~2.5× the time
there (cost as % of period is higher; see the tables), at about a fifth of the energy per callback
(1.7 vs 8.7 µJ in the probe).

### 2.4 Displays

The window opens on the 320 Hz GA27T1M (main display) in every test unless stated. `winctl info`
(CGGetDisplaysWithPoint + CGDisplayCopyDisplayMode) and the app's own log column `display_hz`
(winit's monitor refresh rate) both say 320. Test J moves it to the 60 Hz XB271HU (§8).

### 2.5 Incident: coreaudiod stopped starting streams

At the start of the pass every new output stream hung inside coreaudiod, for every client
including Apple's `afplay`, on every device. Samples showed the client blocked in
`AudioComponentInstanceNew` → `HALC_ProxyIOContext::_TellServerAboutStreamUsage` → `mach_msg`.
coreaudiod still answered property reads, and its TCC checks returned within milliseconds. About
30 minutes later it worked again **with no intervention** (no restart: `sudo` was not available).
The cause is unknown; coreaudiod had been up 37 days. It is noted because an app can't do anything
about it: `build_output_stream` simply blocks. The engine's host thread would block with it, but
the UI wouldn't (the host thread owns stream creation).

## 3. Ground truth for dropouts

### 3.1 cpal's macOS loopback can't be used on this interface

cpal (device.rs, `build_input_stream_raw`) builds a process tap **only if the device has no
inputs**. The MOTU M2 has inputs, so `build_input_stream` on it opens the **physical inputs**. Our
first "loopback" run reported 523 pilot dropouts in 15 s. Recording that stream to a WAV
(`examples/loopcap.rs`) showed why: peak 0.0018, no 19 kHz component, the noise floor of an empty
line input. It worked silently, with no permission prompt, which is what made it dangerous. The
in-app capture (`engine::testkit::Loopback`) now refuses such devices on macOS.

### 3.2 tapmon: a CoreAudio process tap outside the app, and why it isn't ground truth either

`docs/engine/tools/macos/tapmon.swift` creates a `CATapDescription` of everything played to the
device (all processes, stream 0, unmuted, private), an aggregate device holding only that tap, and
an IOProc that runs the same detectors as the Windows loopback: Goertzel over 128-frame blocks
(dropout = a block below 30% of the running level), and click onsets for the A/V test. Events are
stamped in host-clock ns (`mach_absolute_time`, the clock behind Rust's `Instant`), so they line
up with the app's log (`# window_host_ns`) and its A/V marker crossings. It needs the System Audio
Recording permission (granted by the user during the pass).

**Pilot frequency.** With the new Famicom default (no 14 kHz low-pass), the music masks a 19 kHz
pilot. In a clean 60 s run (no stalls; engine late / overload / overrun all 0) the tap counted
**12** dropouts with the Famicom filter and **0** with the NES filter, with detection margins (lowest
clean block ÷ running level, threshold 0.3) of 0.303 and 0.459. With the pilot at **23 kHz**, above
the band-limited synthesis' passband (21.6 kHz), the same Famicom run read **0**, margin 0.566.
Same song, same filter, deterministic render: the 12 were the music, not the audio path. Mac runs
use 23 kHz (`CHIPDAW_PILOT_HZ`, `tapmon --pilot`).

**Validation against injected stalls: failed.** The tap reported a dropout for only 1–3 of the
10–14 stalls per buffer size that CoreAudio itself reported as overloads, plus occasional bursts
of 10–47 "dropouts" from a single stall (stall sweep with `headless --pilot --inject`, tap events
matched by host time). To see why, the tap's samples were recorded around 16 ms and 32 ms stalls
at 512 frames, each reported as an overload, each followed by a 26.8 / 42.8 ms callback gap. **The
tapped pilot is perfectly continuous through every one of them**: no zeros, no phase jump in a
second-order sine predictor. (The only zeros were ~78-sample runs in each period during the first
1.45 s after our stream started: a tap start-up artifact.)

So a process tap records the sample stream a process *delivered*, in order, not what the device
played. When our IO cycle is skipped, the tap simply receives the next buffer. It can't see the
tapped process's own lateness. WASAPI loopback on Windows captures the endpoint mix, which is why
it worked there.

**Conclusion (brief §3.4): there is no valid software ground truth for our own dropouts on this
Mac.** cpal's loopback records the M2's inputs (§3.1), and a process tap is blind to the tapping
target's lateness. **No "0 dropouts" claim in this report rests on the tap.** The tap columns below
are kept only as "dropouts visible in the delivered stream" (e.g. a pilot gap the engine itself
rendered).

**The best alternative, not available this pass:** a physical loopback. A cable from the M2's
output into its own input, captured through cpal's input path (which works on the M2's inputs),
measures what the DAC actually played, fully outside the software stack. Precaution: direct
monitoring off (MONITOR MIX fully to playback), input gain at minimum, speakers down. The user had
no cable during the pass. **Until then, the dropout count on macOS is CoreAudio's own overload
report** (`kAudioDeviceProcessorOverload`, the HAL's statement that our IO missed its deadline),
cross-checked with the engine's late detector.

### 3.3 Detector validation: stall sweep

Stalls injected into the audio callback (`headless --inject`, busy-wait from the callback start,
so the callback lasts that long), one every 0.5 s. "Gap" = the largest gap between callback starts
after the stall. A second sweep with the tap gave the same thresholds to within one step.

| Buffer (period) | Stall | Overrun (callback > period) | Overload (CoreAudio) | Late (engine) | Gap |
|---|---|---|---|---|---|
| 64 (1.333 ms) | ≤ 1.333 | – | – | – | 1.36–1.43 |
| | 1.373–1.467 | ✓ | – | – | 1.47–1.56 |
| | 1.533–1.600 | ✓ | – | ✓ | 1.62–1.69 |
| | ≥ 1.733 | ✓ | ✓ | ✓ | 3.09+ (one cycle skipped) |
| 128 (2.667 ms) | ≤ 2.533 | – | – | – | 2.70 |
| | 2.667–2.827 | ✓ | – | – | 2.82–2.98 |
| | 2.933–3.200 | ✓ | – | ✓ | 3.09–3.36 |
| | ≥ 3.467 | ✓ | ✓ | ✓ | 6.17+ |
| 256 (5.333 ms) | ≤ 5.067 | – | – | – | 5.38 |
| | 5.333–5.653 | ✓ | – | – | 5.64–5.96 |
| | 5.867 | ✓ | – | ✓ | 6.16 |
| | ≥ 6.133 | ✓ | ✓ | ✓ | 11.59+ |
| 512 (10.667 ms) | ≤ 10.133 | – | – | – | 10.76 |
| | 10.667–10.987 | ✓ | – | – | 11.26–11.60 |
| | ≥ 11.307 | ✓ | ✓ | ✓ | 22.06+ |

- CoreAudio tolerates a callback overrunning its period by **0.3–0.8 ms** before it reports an
  overload. Past that it skips a whole IO cycle: the next gap is the stall plus about one period.
- The two signals agree on every stall CoreAudio reports. "Late" also fires in a narrow band just
  below CoreAudio's threshold (64: 1.53–1.60 ms; 128: 2.93–3.20 ms; 256: 5.87 ms), where the next
  callback came late but CoreAudio didn't report a missed cycle. Without a physical loopback it
  can't be settled whether those are audible, so the tables report both.
- Neither fires in clean runs (0 of each in every 20 s clean run at all four sizes).
- **The stall at which overloads begin is the period plus 0.3–0.8 ms** at every buffer size: the
  audio thread may overrun its period slightly, and anything longer costs a whole IO cycle.

## 4. Stress matrix

**Not completed.** The tooling is in place (`docs/engine/tools/macos/matrix_mac.sh`,
`summarize_mac.py`), and short trial runs of A, D, E and G worked (D's window churn: minimise / hide
throttle the UI pump to ~0.55 s ticks, audio unaffected, largest callback gap 5.41 ms; E: §8). The
full run was stopped at the user's direction to fix the flashing first. Completed runs, on the
build before bug 5's fix (which doesn't touch audio):

| Run | Frames / cb | Overloads / late / overruns | Rebuilds | Callback cost % of period: mean / p99 / max | Largest gap | Audio thread: k instr / µJ per callback, P share | Process mW |
|---|---|---|---|---|---|---|---|
| G, 64 frames, 120 s | 64 | **0 / 0 / 0** | 0 | 0.17 / 0.5 / 1.2 | 1.39 ms | 22.9 k · 9.9 µJ · 100% P | 159 |
| G, 128 frames, 120 s | 128 | **0 / 0 / 0** | 0 | 0.18 / 0.3 / 0.7 | 2.72 ms | 35.1 k · 15.2 µJ · 100% P | 156 |

Callback-cost histograms: at 64 frames 89,776 callbacks, all but one below 1% of the period
(48,040 < 0.1%, 12,794 0.1–0.2%, 28,749 0.2–0.5%, 192 0.5–1%, 1 at 1–2%); at 128 frames 44,878,
all below 1%.

**A/V (G):** tap clicks vs presented marker crossings gave −11.41 ms (sd 0.46) at 64 frames and
−12.13 ms (sd 0.20) at 128, but −6.1 ms at 256 frames in test J. If the tap stamped samples before
the output latency, as WASAPI loopback did on Windows, the offset would grow with the buffer, and
it doesn't. The tap's time stamps therefore have no validated zero point, and **these are not A/V
sync results**. A physical loopback (§3.2) would also settle this.

**Next pass:** run `matrix_mac.sh "A B C D G E E256"` on the final build (about 110 minutes),
ideally with a loopback cable for ground truth.

## 5. UI budget with the engine running

PERF-macos.md's own harness (`bench_mac.sh`, chipbench, 12 s per cell, fresh launch per cell),
60 fps cap, two runs each, engine on vs `CHIPDAW_AUDIO=0` in the same session (CPU % on Apple
Silicon is only comparable within a session; PERF-macos.md §5). Instructions per frame is the agreed
stable metric.

| Scenario | Engine off: instr/frame · mW · CPU % | **Engine on**: instr/frame · mW · CPU % | PERF-macos.md (no engine) |
|---|---|---|---|
| Idle | – · 3.7 mW · 0.17–0.18% | – · 4.2–4.3 mW · 0.72–0.75% | 0.1% |
| Demo playing | 15.25–15.28 M · 353–355 mW · 12.2% | **15.52–15.63 M** · 352–356 mW · 12.6–12.8% | 11.0–11.1%, 336–338 mW, 13–15 M/frame |
| 200 clips playing | 16.49–16.53 M · 376–378 mW · 12.6–12.8% | **17.27–17.29 M** · 388–389 mW · 14.1% | 11.7%, 359–362 mW |
| 200 clips, dragging | 19.85–20.13 M · 470–473 mW · 16.6–16.8% | **20.98–21.21 M** · 490–494 mW · 18.5% | 15.5%, 448–452 mW |

- **The engine adds 2–6% instructions per frame and 0–20 mW**: the audio thread (~16 M instr/s,
  ~7 mW at 256 frames on P-cores; §6) plus the UI reading meters and lights. Idle rises from 0.2%
  to 0.7% of a core: the audio thread keeps running while the transport is stopped (it renders
  silence; the chip sleeps, ENGINE.md §2).
- This session's absolute CPU % is ~1 point higher than PERF-macos.md's for the same work
  (engine off: demo 12.2% vs 11.0–11.1%), at the same instructions per frame. That is the clock and
  scheduler variation documented there, not a regression.
- **Bug 3 surfaced here** (§7): before its fix, "200 clips" and "drag" read 7.0 and 11.0–11.4 M
  instr/frame *with* the engine, lower than without it, because F4 left the engine stopped.

**Display-side pacing** (Metal System Trace, `displayed_surfaces.py`, 30 s each, engine on):

| Scenario | App: fps · p99 · max · > 20 ms | Display: updates/s · max hold · > 20 ms | Holds in vblanks (count) | PERF-macos.md (no engine) |
|---|---|---|---|---|
| Demo | 60.0 · 18.05 · 23.02 ms · 1 | 67.0 · 21.9 ms · 4 | **5: 1102, 6: 500**, 1–4: 392, 7: 4 | 5: 1194, 6: 599; > 20 ms: 2 |
| 200 clips | 60.0 · 18.11 · 20.65 · 1 | 66.9 · 21.9 · 3 | **5: 1107, 6: 501**, 1–4: 388, 7: 3 | 5: 1184, 6: 591; 7 |
| Drag | 60.0 · 18.93 · 20.30 · 2 | 66.9 · 53.1 · 17 | **5: 1074, 6: 503**, 1–4: 404, 7: 15, 16–17: 2 | 5: 1184, 6: 595, 16: 2; 3 |

The cadence is the same ideal 5/5/6 pattern. The ~7 extra short updates per second come from the
rest of the screen (PERF-macos.md §4 measured an 8.7/s noise floor from the terminal); they split
holds but never lengthen them. The two ~50 ms holds during a drag are the same as in
PERF-macos.md. **The engine does not change pacing.**

## 6. Instructions and energy per thread

Per-thread figures come from `PROC_PIDTHREADCOUNTS` (§2), validated against a loop of known
instruction count. Measured so far (demo playing, 256 frames, P-cores): the audio thread
executes ~85–89 k instructions and ~38–41 µJ per callback (~16 M instr/s, ~7 mW); the UI thread
~15 M instructions and ~5.7 mJ per presented frame; the whole process ~0.93–0.95 G instr/s at
~360 mW. At 512 frames the audio thread runs on E-cores: ~40 k instr per callback (probe, 440 Hz
tone) at 1.7 µJ instead of 8.7 µJ on P-cores at 256. Full per-run tables need the completed matrix.

## 7. Bugs found and fixed

All three were found by measurement on this machine and fixed in small commits with before/after
numbers. None changes the architecture.

**Bug 1: an overload rebuilt the stream** (`81e9756`).
- *Symptom.* Each audio-thread overrun past CoreAudio's tolerance was followed by ~350 ms of
  silence.
- *Evidence.* cpal reports `kAudioDeviceProcessorOverload` as `ErrorKind::Xrun` through the
  stream's error callback (`mod.rs`, `spawn_overload_listener`); a diagnostic print showed
  `Xrun: A buffer underrun or overrun occurred` from cpal's listener thread at each 8 ms stall. The
  host mapped every unknown error kind to "stream invalidated", dropped the stream, waited 250 ms
  and rebuilt it.
- *Fix.* Count `Xrun` in `Shared::overloads` and keep the stream.
- *Before / after* (stalls of 2, 4, 6, 8, 12 ms at 256 frames, two runs each): 3 rebuilds, largest
  callback gap **352 / 349 ms** → 0 rebuilds, 3 overloads counted, largest gap **17.4 ms**. The gap
  measurement itself had to be fixed first (`7f2802f`): it was reset with each new stream, so the
  rebuild silence never showed.
- *Windows:* not reachable there (WASAPI doesn't report `Xrun`), which is why it wasn't seen.

**Bug 2: "late" fired on normal CoreAudio jitter** (`55e040a`).
- *Symptom.* 1833 and 1945 "late" callbacks in two clean 20 s runs at 256 frames (~95/s).
- *Cause.* "Late" means a gap longer than the largest callback seen (the whole device buffer on
  WASAPI, whose first callback gets all of it). CoreAudio's callbacks are always exactly one
  period, so the threshold was one period, and the measured ±2.3% scheduling jitter crossed it.
- *Fix.* When no callback has exceeded the period, allow 1/8 of it (at least 0.25 ms). WASAPI's
  threshold is unchanged.
- *Before / after:* ~95/s → **0** in clean 20 s runs at 64, 128, 256 and 512 frames. It still fires
  on every stall that CoreAudio reports as an overload (§3.3).

**Bug 3: F4 showed "playing" while the engine stayed stopped** (`fbc2309`; also on Windows).
- *Symptom.* The play button lit and the playhead moved, but no sound, idle meters and lights.
- *Evidence.* With the engine on, the 200-clip and drag benchmarks used fewer instructions than with
  it off. The log column `playhead_local_frames` showed the playhead on the UI's fallback clock
  on **60 of 60** frames per second.
- *Fix.* `ToggleStress` sends `Seek` + `Play`, like the play button.
- *Before / after:* 60/60 → **0/60** local frames per second; "200 clips" 0.42 → 1.04 G instr/s.
- ENGINE.md §7's UI-budget row "200 clips playing" was very likely measured with the engine
  stopped. The matrix test B was not affected (it starts through `CHIPDAW_STRESS`, which sends
  `Play`).

**Bug 4: a device change kept the stream on the wrong buffer size, with blind detectors and no
message** (`fcfdca7`; found by test H).
- *Symptom.* After unplugging the M2 (or any default-output change), CoreAudio rerouted the
  running stream by itself and the host ignored it (`453afc5`, written before this pass).
- *Evidence* (test H, before the fix): callbacks went from the requested 256 to the new device's
  own 512 frames (latency 5.9 → 14.4 ms) and stayed at 512 after re-plugging. The detectors kept
  their 256-frame baseline: ~45 false "late" per second for the rest of the run, and in the
  scripted switch at 256 frames 656 false "late" and 1408 false "starved". The F3 overlay still
  said "M2" while the speakers played, and the app showed no message.
- *Fix.* On `DeviceChanged` the host rebuilds the stream on the new default at once (no 250 ms
  wait), which reapplies the buffer size, resets the detectors and updates the device name. The
  transport bar shows "AUDIO: now playing on <device>" for 5 s.
- *Before / after* (default output M2 → speakers → M2 at 256 frames): buffer on the speakers 512 →
  **256**; false late / starved 656 / 1408 → **0 / 0**; gaps 51 / 84 ms → 74 / 164 ms. The rebuild
  costs 25–80 ms of extra silence per change: the price of keeping the user's buffer setting.

**Bug 5: the window flashed after moving to another display** (vizia fork `54766d77`, app
`1066910`; found by the user in test J).
- *Symptom.* On the 60 Hz display "some of the clips … flash slightly". Not on the 320 Hz display.
- *Evidence.* `docs/perf/tools/macos/flashcap.swift` captures the window with ScreenCaptureKit at
  full rate and flags blocks that change and revert in the next frame. At a fine threshold it found
  whole-window events: every flagged frame differed from its neighbours by the *same* colour shift
  (mean RGB +3.5/+2.2/−0.7, up to +102 in red on the pink clips). That's one picture shown
  through two colour conversions.

  | Window on the 60 Hz display (20 s) | Whole-window flips |
  |---|---|
  | cap 60 (default) | 38–67 |
  | cap off, vsync on (~119 fps) | 975 |
  | vsync off, uncapped | 0 |
  | cap 60, vsync off | 67 |
  | after a resize on the 60 Hz display | **0** |
  | on the 320 Hz display (where it opens) | 0 |

  Calling `NSOpenGLContext.update()` on the display change (Apple requires it, and fork commit 2
  had stopped it running every frame) changed nothing: 10–24 / 987.
- *Cause.* AppKit tags the window with the colour space of the display it is on. After the move,
  frames were colour-matched inconsistently: some as for the 320 Hz display's wide-gamut profile,
  some as for the 60 Hz display's.
- *Fix.* Pin the window's colour space to sRGB at creation (`NSWindow.colorSpace`), which is what
  Skia renders: fork commit 8, through the Objective-C runtime, no new dependency.
- *Before / after:* 38–67 (cap 60) and 975 (uncapped) flips per 20 s → **0** in every
  configuration, after repeated moves both ways; **user confirmed by eye: "gone"**.
- *Side effect:* colours are now properly converted on wide-gamut displays. On the 320 Hz panel
  one clip colour went from RGB 90/42/59 to 81/47/59 (slightly less saturated); greys and the
  background are unchanged, and nothing changed on the sRGB-like 60 Hz panel. Pinning to "device
  RGB" instead would keep the old look but leave colours unmanaged. The PERF-macos.md screenshots
  predate this.

## 8. Tests that need the user (H, I, J)

All three were done with the user at the machine, playing at 256 frames.

| Test | What happened | Result |
|---|---|---|
| **H. Unplug** the M2, wait 15 s, plug it back | *Before bug 4's fix:* sound moved to the Mac mini speakers and back, no crash, song and position kept, playhead on the audio clock throughout. But no message, the buffer fell back to 512 frames, false detector counts. *After:* sound moved and came back, **"AUDIO: now playing on Mac mini Speakers" / "… on M2" shown** (user confirmed), 256 frames throughout, 0 false detections | **Pass after fix.** Gap: **80 ms** on unplug, **713 ms** on re-plug (before the fix 56 / 615 ms). The re-plug gap is mostly CoreAudio bringing the M2 back up |
| **I. Sleep / wake** (Apple menu → Sleep, ~1 min, wake) | No crash or hang. The same stream resumed by itself at 256 frames (no rebuild, no error), the playhead stayed on the audio clock. macOS's monotonic clock (`Instant`) doesn't advance while asleep, so playback simply continued where it paused (one 550 ms gap in the log). **13 s after wake, audio callbacks stopped for 7.6 s** while the UI kept running normally: the displays' audio devices (DisplayPort, HDMI) were dropping and returning as the monitors woke (`coreaudio watch`), and CoreAudio paused our IO meanwhile. cpal reported nothing; the playhead froze correctly and resumed in sync. The monitor reconnect also moved the window to the 60 Hz display | **Pass**, with a finding: a silent 7.6 s pause after wake with no message. A "no callbacks for > 1 s while the stream should run" notice would cover it (not built: UI) |
| **J. Display move** (320 Hz ↔ 60 Hz, dragged by the user, then repeated with `winctl move`) | Audio: 0 overloads, 0 late, largest callback gap 5.40 ms; playhead always from the audio clock; the app's `display_hz` followed the window both ways. **Pacing:** on the 60 Hz display the 60 fps cap beats against the display's real rate (59.951 Hz; vizia reports the mode's 60.0): every ~20 s a 5–10 s stretch of 2-vblank (33 ms) frames. 118 frames > 20 ms in 119 s on the 60 Hz display, against 0–2 per 30 s on the 320 Hz display. **A/V** (one-bar click loop, tap clicks vs presented crossings): 320 Hz −6.16 / −6.10 ms (sd 0.09 / 0.07) before / after the move; **60 Hz −6.71 ms but sd 4.84, outliers to −29 ms**. **User report: "on the 60 Hz display some of the clips start to have a visual glitch and will flash slightly"**; only there, not on the 320 Hz display | Pacing on the 60 Hz display: **fixed (stopgap), see PERF-macos.md §11** (fork commit 9: 230 → 6 frames > 20 ms per 120 s; the ≤ 2 target is not met, the rest are isolated 3-vblank holds that predate the fix). The flashing was a separate bug, **fixed** (bug 5). Audio unaffected |

**J, what was tried and what's proposed.**
- *Tried:* give up fixed slots when the display is no faster than the cap and let vsync pace
  frames. The beat vanished (118 → 1 frame > 20 ms), but on macOS `flushBuffer` doesn't lock to
  vblank (PERF-macos.md §4), so the app presented **119 fps** on the 60 Hz display, twice the work
  for nothing. **Reverted.** The clip flashing was still there with it (user checked), so the
  flashing is a separate problem from the beat.
- *Proposed fix for the beat:* pace frames from the display link (`CVDisplayLink` /
  `CAMetalDisplayLink`) of the window's current display, i.e. vblank-locked slots, instead of a
  free-running 60.0 Hz clock. That's the change PERF-macos.md §6/§7 already recommends together
  with the Metal backend; it belongs to vizia's winit backend (out of scope here).
- *Stopgap, done:* fork commit 9 locks the slots to the display's precise rate (59.951 Hz) and
  re-anchors their phase to display-link timestamps; the display link never drives rendering.
  See PERF-macos.md §11.
- *The flashing clips:* found and fixed (§7, bug 5). It was unrelated to the beat.

## 9. Possible upstream issues (cpal 0.18.2, CoreAudio)

1. **Loopback silently records the inputs on devices that have them.**
   `build_input_stream` on an output device builds a process tap only when
   `!self.supports_input()`. On any interface with inputs (most USB audio interfaces) the same
   call opens the inputs with no error, so "record what this device plays" gets the wrong signal.
   *Proposal:* an explicit loopback request (e.g. a `Device::loopback()` or a stream option)
   rather than inferring it from the device's directions.
2. **`OutputStreamTimestamp::callback` is CoreAudio's output time, not the callback time.**
   Measured `callback − now` = one period + the safety offset at every buffer size (§2.2), and
   `playback` adds the buffer again, so the absolute `playback` instant is one buffer late. The
   docs say `callback` is "the instant the stream's data callback was invoked" and `playback` "the
   predicted instant that data written will be delivered to the device for playback".
   *Proposal:* use the current host time for `callback` (or document it as the output time and
   stop adding the buffer to it).
3. **`Xrun` arrives through the same error callback as fatal errors.** Its docs say "a buffer
   underrun or overrun occurred, causing a potential audio glitch", but not whether the stream is
   still running. A host that rebuilds on unknown errors turns a one-cycle glitch into a rebuild
   (bug 1). *Proposal:* say in the `ErrorKind::Xrun` docs that the stream keeps running.

## 10. Comparison with Windows (so far)

| | Windows (ENGINE.md) | macOS (this pass) |
|---|---|---|
| Buffer request | ignored: always 512 frames (10.67 ms), lead 34 ms | **honoured exactly**: 64 / 128 / 256 / 512, latency = buffer + 0.58 ms |
| Audio thread | WASAPI event thread, normal priority | CoreAudio IO thread, round-robin priority 97, time-constrained; E-cores at 512 frames |
| Ground truth | WASAPI loopback (validated) | none without a cable; process tap blind to own lateness (§3) |
| Underrun signal | "starved" (callback > one period), validated | CoreAudio overload report (cpal `Xrun`), plus "late" |
| Overload handling | — | was a ~350 ms stream rebuild (bug 1) |
| Default-device switch | rebuild, ~0.4 s gap | rebuild (bug 4), 74–164 ms gaps; unplug 80 ms, re-plug 713 ms |
| Callback cost | p99 ≤ 0.6% of 10.67 ms | p99 ≤ 0.5% even at 64 frames (1.33 ms) |
| UI budget with engine | +0.5 points of a core | +2–6% instructions/frame, same cadence |
| Display issues | — | 60 fps cap beats against 59.95 Hz displays (fixed, stopgap: PERF-macos.md §11); window flashing on display change (fixed) |

## 11. Lowest reliable buffer size and latency

Provisional: **64 frames** played 120 s of the A/V test with 0 overloads, 0 late, callback p99 0.5%
of the period, and the stall sweep shows CoreAudio tolerating callbacks up to ~0.3 ms past the
1.33 ms period. Output latency 1.9 ms (64 + 28 frames at 48 kHz). That makes **128 frames
(3.3 ms)** the conservative choice for live keyboard input. Neither is confirmed by the A/B/C stress
runs, which weren't completed, nor by a physical loopback.

## 12. Verdict (provisional)

The engine is sound on macOS once five bugs are fixed: CoreAudio gives it what WASAPI couldn't
(real small buffers on a real-time thread), and it costs under 1% of even a 1.33 ms period. The
remaining macOS issues are not in the engine: there is no software ground truth for dropouts
(needs a loopback cable), and frame pacing on 60 Hz displays needs vblank-locked frames (display
link; the PERF-macos.md Metal recommendation). Finish the stress matrix on the final build before
calling it done.
