# cpal issue write-ups (cpal 0.18.2)

Found while building ChipDAW's real-time engine (ENGINE.md, ENGINE-macos.md). Each is written as
it would be filed. Evidence is from our probes (`crates/engine/examples/probe.rs`, `probe_mac.rs`).

---

## 1. macOS: building an input stream on an output device records its *inputs* when it has any

**What happens.** On CoreAudio, `build_input_stream` on an output device builds a process tap
(loopback) only when `!device.supports_input()`. On a device that also has inputs (most USB audio
interfaces, e.g. a MOTU M2), the same call opens the inputs without any error. "Record what this
device plays" silently records the microphone or line inputs instead.

**Expected.** A way to ask for loopback explicitly (e.g. `Device::loopback()` or a stream config
flag), or an error when loopback isn't what will be built.

## 2. macOS: `OutputStreamTimestamp::callback` is the output time, not the callback time

**What happens.** Measured `callback − now` equals one buffer period plus the device safety offset,
at every buffer size (64 to 512). `playback` adds the buffer again, so `playback` is one buffer
late.

**Docs say:** `callback` is "the instant the stream's data callback was invoked", and `playback`
is "the predicted instant that data written will be delivered to the device for playback".

**Expected.** Use the current host time for `callback`, or document that it's the output time and
stop adding the buffer to `playback`. A/V sync code that trusts the docs is a buffer off.

## 3. `ErrorKind::Xrun` docs don't say the stream keeps running

`Xrun` ("a buffer underrun or overrun occurred, causing a potential audio glitch") arrives through
the same error callback as fatal errors. A host that rebuilds its stream on unknown errors turns a
one-cycle overload into ~350 ms of silence (ENGINE-macos.md bug 1). **Ask:** state in the docs that
the stream is still running after an `Xrun`.

## 4. Windows (WASAPI shared mode): the requested buffer size is silently ignored

`BufferSize::Fixed(128 | 256 | 512)` all give the device period: 480 frames on an NVIDIA HDMI
endpoint, 512 on a MOTU, plus a larger first callback (1056–1126 frames). There is no error and no
way to find out. cpal's own source comment says the period "is always GetDevicePeriod() regardless
of what is requested here".

**Ask:** document this on `BufferSize` and `SupportedBufferSize` for WASAPI shared mode, or return
the granted size. Optionally support `IAudioClient3::InitializeSharedAudioStream`, which gives
small shared-mode periods on Windows 10+.

## 5. Windows: MMCSS registration needs an MPL-2.0 dependency

The audio thread runs at normal priority unless the `realtime` feature is on, and that feature pulls
in `audio_thread_priority` (MPL-2.0). For projects limited to MIT/Apache, the only option is their
own `AvSetMmThreadCharacteristicsW` FFI. **Ask:** an MIT/Apache path (a few lines of `windows`
crate calls) behind a feature. ChipDAW ran 0 dropouts at normal priority on a fast machine
(ENGINE.md §7), but slower machines may need it.
