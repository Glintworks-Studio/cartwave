# POLISH2.md: track header, layout and navigation

Branch **`polish2/header-nav`** (from `main` 8644ff3). Prompt: `POLISH2_PROMPT.md`. Decisions:
[`docs/RUNLOG.md`](docs/RUNLOG.md) P2-D1…D17. Spec: [`docs/design/VISUAL-SPEC.md`](docs/design/VISUAL-SPEC.md)
§4, §4a, §10, §11; [`UX.md`](UX.md) §1, §2, §4c, §4d, §5.2, §6. Reference pack:
[`docs/design/polish2-ref/`](docs/design/polish2-ref/). Manual citations:
[`docs/polish2/manual-refs-p2.md`](docs/polish2/manual-refs-p2.md) (P2-R1…R44).

## 1. Summary (one screen)

@@SUMMARY@@

## 2. What changed

- **The E1 track header** (§3): a 120 px control block at the edge toward the clips, two columns
  16 px apart. Column A: M / S (unchanged), the pan value and an editable pan slider. Column B: a
  vertical volume meter-fader **in dB** (−inf … +6, position `(dB + 48)/54`, meter stops
  0/45/70/85/89 %, hot at 0 dB) and its dB value. **Double-click a value to type it** (dB: `-3`,
  `-3 dB`, `−3`, `+2.5`, `-inf`; pan: `C`, `0`, `25L`, `30R`, `-25`, `+25`): Enter commits, Esc
  cancels, a click away commits; clamped; one undo step; the unsaved mark.
- **Track volume is a real gain** (P2-D4): the old fader moved the chip's 4-bit Volume register,
  capped at unity. Each track now has a dB volume applied after the chip (`chip::mixer::mix_gained`);
  at 0 dB the old mix runs unchanged, so the golden render and every migration hash are
  bit-identical. Every track has a working fader (the triangle and DPCM too). The register stays
  the inspector's Volume dial.
- **Pan** is editable and saved, heard once stereo mixing arrives (Batch 4); its help says so.
- **Format v6** (P2-D5): row heights, pan and volume per track; v5 files open unchanged
  (`docs/songs/v5` corpus, hashes equal).
- **The strip** is toned down ~20% (white mixes .28/.55, glow .40, spill .25) in the palette's
  `strip` recipe; the palette sheet is regenerated.
- **Row heights** (§4): 72 … 300 px in 4 px steps: drag a track's bottom edge (Alt: every track)
  or Alt+wheel. Taller rows keep the 72 px band at the top; only the fader stretches; dB scale marks
  from 96 px.
- **Header width, side and order** (§5): the boundary drags 380 … 600 (extra width to the name);
  **Settings ▸ Look & Feel**: Track headers Left/Right, Colour strip Outside/By the clips, Browser
  Left/Right. The four orders of the table; the TRACKS and OVERVIEW cells move with the headers.
- **The status bar** (§6): messages (8 s, then "Saved · name"), persistent problems (an icon until
  resolved; a click lists them), hover help on the right. Every toast moved there
  ([`docs/polish2/notices.md`](docs/polish2/notices.md): 62 message sites); dialogs stay for decisions.
- **Navigation** (§7): the ruler zooms (vertical drag) and scrolls (horizontal drag); a click still
  places the playhead; Shift+drag scrubs; **W** fits the song (again: back); **the Overview** (32 px,
  cached clip layer, drag/zoom/jump/double-click, View ▸ Show Overview, Ctrl+Alt+O).

@@DETAILS@@
