# Rename gate: exports (RENAME_PROMPT.md §1.5, §3.1)

`docs/rename/tools/export_rn.sh`: `crates/ui/examples/game_export.rs` run by the baseline (main 8e6b4f8, `../before-rename`) and `rename/cartwave` on the same inputs. WAVs compared byte for byte; each JSON compared after mapping its one renamed field back (`"format": "cartwave-loop"` → `"chipdaw-loop"`), so any other difference would show.

| Input | Files | WAVs identical | JSON (after mapping the format field) |
|---|---|---|---|
| showcase: `docs/songs/showcase.mid docs/songs/hey.wav` (--rate 44100 --loop-bars 8 --stems channels --name showcase) | 28 | **all identical** | only `format` differs |
| studio: `docs/songs/v7/v7_studio_fader_voices.chipdaw` (--stems channels --name studio) | 28 | **all identical** | only `format` differs |
| demo: `docs/songs/v7/demo_band.chipdaw` (--stems groups --name demo) | 13 | **all identical** | only `format` differs |
| panned: `docs/songs/v7/v7_studio_panned.chipdaw` (--stems channels --name panned) | 28 | **all identical** | only `format` differs |
| v2demo: `docs/songs/v2/demo_band.chipdaw` (--bits 24 --rate 48000 --stems none --name v2demo) | 4 | **all identical** | only `format` differs |

**The audio bytes are identical** (101 files, every WAV). The WAV writer has no software-name chunk (no `LIST/INFO`), so no WAV changes at all. The one metadata-only difference is the loop JSON's `format` field (RN-D7).
