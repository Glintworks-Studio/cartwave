# Gate 1: the game export without pan, byte for byte (Batch 4b §9.1)

> *Historical report, kept as written under the working title ChipDAW. Since the rename (2026-11-27, [RENAME.md](../../RENAME.md)) the app is Cartwave by Glintworks: projects save as `.cartwave`, the executable is `cartwave`, and environment variables are `CARTWAVE_*` (the `CHIPDAW_*` names still work).*

`crates/ui/examples/game_export.rs` run by the baseline (main d4f1afd, ../before-batch4b) and
this branch on the same inputs; every file compared byte for byte.

| Input | Options | Files | Result |
|---|---|---|---|
| `docs/songs/showcase.mid` + `hey.wav` (4a's showcase) | 44.1 kHz, loop the last 8 bars, stems per channel | 31 (JSON + 30 WAVs) | **identical** |
| `docs/songs/v7/v7_studio_fader_voices.chipdaw` (studio tracks, faders, voice caps, no pan) | stems per channel | 31 | **identical** |
| `docs/songs/v7/demo_band.chipdaw` (the demo, locked tracks) | stems per group | 13 | **identical** |

With nothing panned the export stays mono and its JSON unchanged (B4B-D6). The stereo case is
pinned by `tests/batch4b.rs` (`b4b_exports_are_mono_until_a_pan_and_stems_follow_the_mix`,
the stereo golden hash).
