# Batch 5 A3: the intentional re-baseline (studio voices made bipolar)

**Why these hashes change.** A studio voice used to add its channel's DAC level relative to the
power-up level: unipolar, so each note carried a DC offset (a pulse's duty × its level) that
the studio bus's only DC filter (a 5 Hz one-pole blocker in a studio-only project, τ ≈ 32 ms)
couldn't remove fast enough. The baseline jumped at every note start and stop (listening pack:
a 50 ms running mean swinging up to 0.18 of full scale; Supersquare's peaks −0.09 / +0.34).
From this commit each voice also subtracts its waveform's own mean (`chip::voice::Voice::dc`,
RUNLOG B5-D6), so every **studio** track's samples change. Nothing else does:

- **Locked tracks are bit-identical everywhere.** The console never uses `Voice`. Checked on
  every project file in the repository (299), rendered by `main` and by this build with every
  studio track muted: **299 of 299 identical** (`docs/batch5/identity/a3/`, the `console`
  column of `corpus_hashes`).
- **Files without a studio track are bit-identical:** 236 of 299 files are identical in render,
  export and channel count; the 63 that changed all have at least one studio track
  (`corpus-changed.txt`, its second column).
- **Game export:** the three songs without studio tracks (showcase, demo, v2demo: 45 files)
  are byte-identical; of the studio song 27 of 28 files differ (its JSON is identical), of the
  panned song only the panned studio lead's mix and stem (6 of 28).

These goldens and migration hashes changed in this commit only (each one is a studio-track
file):

| Test | Value | Old | New |
|---|---|---|---|
| `crates/ui/tests/batch4b.rs` `STEREO_GOLDEN` | the panned showcase's stereo render | `0x3e54_3873_23b0_f321` | `0x3a95_c377_6a9b_c7d3` |
| `migration.rs` `a_v7_panned_studio_track_is_heard_now` | `v7_studio_panned` centred, export | `0x1b1742f7c9b4dd4d` | `0x110ab1b1ebd43407` |
| `migration.rs` `V7` | `v7_studio_demo` render / export | `0xfaa80d8bb5a43098` / `0xdecd97a4045f332c` | `0x22285f6d9072481c` / `0x8a6792789e9b2cbe` |
| `migration.rs` `V7` | `v7_studio_fader_voices` render / export | `0xb6fe5013dffff23b` / `0xd1ea1ac50678eab3` | `0x72d142f12825b6c2` / `0x1e081445b8abe424` |
| `migration.rs` `V7` | `v7_studio_showcase` render / export | `0x5ee888eb6ce72c4b` / `0x3943be55e2b424f7` | `0xfab8c43558778610` / `0x9fb5eecddcf833f6` |
| `migration.rs` `V8` | `v7_studio_demo` render / export | `0xfaa80d8bb5a43098` / `0xdecd97a4045f332c` | `0x22285f6d9072481c` / `0x8a6792789e9b2cbe` |
| `migration.rs` `V8` | `v7_studio_showcase` render / export | `0x5ee888eb6ce72c4b` / `0x3943be55e2b424f7` | `0xfab8c43558778610` / `0x9fb5eecddcf833f6` |
| `migration.rs` `V8` | `v8_studio_unison` render / export | `0x7e960f096a5aafdd` / `0x65a6684d34702df3` | `0x5885739af003e8d1` / `0x3c8c18c309b815ab` |
| `migration.rs` `V8` | `v8_studio_unison_panned` render / export | `0x1f4b716b2b275954` / `0x66de0d77b635b17a` | `0x4b5e115b1b2cc7be` / `0x96af9360943984cb` |

Unchanged in the same tables: `demo_band`, `showcase`, `layout_2`, `scale_test`,
`v6_rows_pan_mute`, `v6_vrc6_unused`, `v8_arps` (no studio tracks), the golden render
(`golden.rs`) and every chip test except the new `studio_voices_are_bipolar`.

The listening pack (`docs/audio/presets/`, its `levels.md`) was re-rendered in the same commit;
the before/after table for the whole bank is `docs/batch5/a3-dc.md`.
