# Export examples: the showcase as a game would use it

Made with `cargo run --release -p ui --example game_export -- --rate 44100 --loop-bars 8 --stems channels --name showcase docs/songs/showcase.mid docs/songs/hey.wav <dir>`
(the song is 16 bars at 140 BPM; the last 8 bars loop). 44.1 kHz, 16-bit mono, authentic level.
See [`../EXPORT.md`](../EXPORT.md).

| File | What |
|---|---|
| `showcase.wav` | One file with the loop in its `smpl` chunk (loop = samples 1209598..1814397) |
| `showcase_intro.wav` + `showcase_loop.wav` | Play the intro once, then repeat the loop |
| `showcase.json` | Loop points and tempo |
| `showcase_stem_<channel>_loop.wav` (8) | Each channel's stem of the loop, same length and alignment as `showcase_loop.wav` |

To keep the repository small, only the stems' loop files are committed. The command above also
writes each stem's single file and intro.
