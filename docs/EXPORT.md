# EXPORT: music for games (loops, stems, loudness)

**EXPORT** (Ctrl+Shift+R) opens the export dialog. Everything is rendered offline by the same
2A03 + VRC6 emulation you hear while editing, on a worker thread: the UI stays responsive, and the
live audio thread is never touched. A typical song (40 s, mix + 8 stems) renders in under a second.

Command line, same code: `cargo run --release -p ui --example game_export -- [options] song.chipdaw out_dir`
(options in `crates/ui/examples/game_export.rs`). `docs/export-examples/` was made with it.

## 1. Options

| Option | Choices | Notes |
|---|---|---|
| Range | **Song** (start to the loop end, + 2 s release tail) · **Loop region** (seamless loop) · **Selection** (the time selection, + tail) | Loop region uses the song loop (the brace in the ruler) |
| Format | 44.1 kHz or 48 kHz · 16 or 24 bit · mono | Rendered natively at either rate (no resampling). The console's output is mono |
| Loop files | One WAV with loop points · intro + loop WAVs · JSON sidecar | Loop region only; any combination |
| Stems | Mix only · 8 channels · groups (default melody / bass / drums; click a lane to move it) | Same length and loop points as the mix |
| Loudness | **Authentic chip level** (default) · peak −1 dBFS · −16 LUFS integrated with a −1 dBTP true-peak ceiling | One gain, measured on the mix, applied to the mix and every stem. The emulation itself is never changed |

## 2. The files (loop region export of `song`)

| File | What it is |
|---|---|
| `song.wav` | Everything up to the end of the loop's **second** pass. A `smpl` chunk holds one forward loop over that second pass (start inclusive, end inclusive as the format defines it, infinite repeat); `cue ` points and `LIST/adtl` labels mark "Loop Start" and "Loop End" |
| `song_intro.wav` | Everything **before** the loop's second pass: the intro **and the first pass** (see §3) |
| `song_loop.wav` | The second pass: repeat it forever |
| `song.json` | `loop_start_sample`, `loop_end_sample` (exclusive), `loop_length_samples`, `intro_samples`, `musical_loop_start_sample` (where the intro proper ends), `sample_rate`, `bits`, `bpm`, `loop_start_bar` / `loop_end_bar` / `bars_in_loop`, `gain_db`, and the file names |
| `song_stem_<name>.wav`, `..._intro.wav`, `..._loop.wav` | The same for each stem |

All sample positions are in samples at the file's rate, counted from 0.

## 3. Why the loop is seamless, and why "intro" includes the first pass

An emulator never repeats itself exactly. The music driver's 60.1 Hz clock, the noise channel's
LFSR, the oscillator phases and the output filter all keep running through a loop. Without care,
the third pass would differ slightly from the second, and a WAV loop would click or shift.

The export does this:

1. It renders the intro and the first pass normally.
2. At the start of the second pass it copies the **entire** emulator state.
3. It renders every later pass from that copy.

The loop is then exactly periodic. Its start contains what continuous playback has there: release
tails that rang past the loop end, envelopes, filter state, the LFSR. **Tested:** `intro + loop × 3`,
read back from the files, is sample-identical to one continuous render of the same length. This
holds at 44.1 and 48 kHz, 16 and 24 bit, for the showcase and the test song
(`crates/ui/tests/export_loop.rs`).

The *first* pass follows the intro, not the loop end, so its start has the intro's tails instead.
That's why the part played once (`song_intro.wav`, or `song.wav` up to the loop start) runs up to
the second pass. If your song loops from the very beginning (no intro), `song_intro.wav` is one
full pass, and `song_loop.wav` is the same music with the loop-end tails at its start.

## 4. Setting up the loop in an engine

**General method (any engine or audio middleware):**

- **Two files:** play `song_intro.wav` once, then queue `song_loop.wav` on loop, sample-accurately
  (a scheduled or "play after" API, not a timer). Any gap between them is a gap in the music.
- **One file:** play `song.wav` from 0. When playback reaches `loop_end_sample`, jump to
  `loop_start_sample`. Use the engine's loop-point fields if it has them (values from the JSON or
  the `smpl` chunk), or a streaming callback that wraps the read position.
- Don't let the engine resample or trim silence, and keep the files uncompressed or losslessly
  compressed. Lossy encoders (MP3, AAC) pad the start and end of a file, which breaks
  sample-accurate loops unless the encoder and decoder handle gapless metadata.

**Godot 4** (verified in the official docs, "Importing audio samples", Godot 4.7). For WAV files
the importer's *Edit > Loop Mode* has **"Detect from WAV"**, which "uses loop information from the
WAV metadata", and that's the default behaviour. *Loop Begin / Loop End* are set in samples from
the start of the file (Loop End −1 means the end of the file). With `song.wav`, check that the
import dock shows Loop Begin = `loop_start_sample` and Loop End = `loop_end_sample` from the JSON.
If the loop mode reads "Disabled", set Forward and type those two numbers in.

Other engines: their documentation wasn't verified for this pass, so use the general method above.

## 5. Stems, and why they don't add up exactly

The 2A03 mixes its channels **non-linearly**: pulse 1 + 2 go through one curve, and triangle,
noise and DPCM through another. A loud channel makes the others relatively quieter. A solo render of
each channel therefore doesn't sum to the full mix. Measured on the showcase (48 kHz, whole export):

| Stems | Sum − mix, RMS (relative to the mix) | Peak error |
|---|---|---|
| 8 channel stems | **−11.3 dB** | −20.3 dBFS |
| 3 groups (melody / bass / drums) | **−11.5 dB** | −19.3 dBFS |

Each group is rendered through the real mixer with the other lanes silent. Groups still don't sum
exactly, because the two mixer curves interact across groups. For an interactive soundtrack (fading
layers in and out), that is the authentic behaviour of the chip. If you need the full mix, use
the mix file, not the summed stems.

## 6. Loudness

- **Authentic** keeps the console's level: a single pulse peaks around 0.15 of full scale and a
  full mix around 0.6. Choose it when the game mixes its own levels.
- **Peak −1 dBFS** applies one gain so the loudest sample reaches −1 dBFS.
- **−16 LUFS** measures integrated loudness (EBU R128, `ebur128` crate) and applies one gain to
  reach it. If that would push the true peak over −1 dBTP, the gain is lowered so the ceiling holds,
  and the report says how loud the file ended up.

No compressor or limiter is used, ever: the gain is a plain multiplication. Stems get the mix's
gain so their balance is kept, and the loop stays sample-exact.

## 7. Known limits

- **Live playback vs the exported loop.** In the app, each loop pass re-quantises notes to the
  60.1 Hz frame clock, so on later passes some notes can land one frame (16.6 ms) earlier or later
  than in the export. In the showcase the loop is 824.21 frames long, so the frame phase moves 0.21
  frame per pass. The export keeps the second pass's timing, which is also what the app plays on
  its second pass.
- Mono only (as the hardware is). Dither isn't applied when going to 16 bit.
