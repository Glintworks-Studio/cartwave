# Listening pack

Rendered offline by the same emulator the app plays in real time (`crates/chip`), 48 kHz,
16-bit mono, band-limited. Levels are **hardware-relative**: nothing is normalised, so a single
pulse peaks around 0.15 of full scale and the full mix around 0.6, like the console's own output.
Turn your monitors up rather than comparing loudness between files.

Each chip-level file comes in two versions:

- `*_nes.wav`: through the NES analog output stage (first-order high-pass 90 Hz + low-pass 14 kHz,
  ENGINE.md §3.4).
- `*_raw.wav`: no console filtering (only a 5 Hz DC blocker).

Compare both against hardware recordings. Real consoles differ (front-loader, top-loader,
Famicom, AV mods), and so do their capture chains.

| File | Content | Listen for |
|---|---|---|
| `pulse_duty_sweep_*` | Pulse 1: duties 12.5 / 25 / 50 / 75% at A3 and A4. Then hardware sweep downwards until the overflow mutes it, sweep upwards until period < 8 mutes it. Then envelope decay at V = 2, 6, 12 | 25% and 75% should sound identical (75% is 25% inverted). The sweeps end abruptly (sweep muting) |
| `triangle_bass_*` | Triangle E1..D2 bass line, odd steps staccato through the linear counter. Then a note "silenced" by writing period 0 | The 4-bit staircase timbre. The pop when period 0 is written (ultrasonic output averaging to ~half level) |
| `noise_modes_*` | Noise: long mode, periods 0..15, then short (periodic) mode, periods 0..15 | Short mode is metallic and pitched. The lowest periods in long mode are rumbly |
| `vrc6_saw_*` | VRC6 saw: A1 A2 E2 D2 at accumulator rate 42 (full), 20, then 50 (wraps: distorted) | The rate-50 notes should distort (accumulator overflow) |
| `vrc6_pulse_duties_*` | VRC6 pulse 1 at A3, duty 1/16 .. 8/16 | Eight distinct timbres |
| `dpcm_kit_*` | Our synthesised DPCM kick, snare, tom, crash at rate 15 (native), 12 and 9 | 1-bit delta-modulation grit. Pitch drops at lower rates. **These samples are our own, made for this test, not taken from any game.** |
| `aliasing_naive.wav` vs `aliasing_bandlimited.wav` | 12.5% pulse arpeggio C6..G8, NES filter. Naive point sampling vs our band-limited synthesis | The naive file has inharmonic "whistles" moving against the notes; the band-limited one doesn't |
| `demo_excerpt_*` | The app's demo arrangement, bars 1-16 at 150 BPM, all 8 channels through the sound driver (frame-tick timing) | Overall balance, including the VRC6 level (ENGINE.md §6.5, gap 1) |

Regenerate:

```sh
cargo run --release -p chip --example lab -- pack          # chip-level files
cargo run --release -p app -- --render-demo docs/audio 25.6
```
