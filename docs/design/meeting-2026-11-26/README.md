# Reference pack: the 2026-11-26 meeting

The CEO's boards from the 2026-11-26 meeting, recorded in [VISION.md](../../../VISION.md),
[IDEAS.md](../../../IDEAS.md) and [VISUAL-SPEC.md](../VISUAL-SPEC.md) by the docs update
([DOCS-2026-11-26.md](../../../DOCS-2026-11-26.md)). Each board is a static HTML file (1 CSS px =
1 logical px) with PNG renders at 1× and 2×. "The canvas board" in those documents means the
matching file here.

| Board | Status | What it fixes |
|---|---|---|
| `mixer.*` | **Locked** | The mixer view: channel-budget bar, two-digit number slots, wide scrolling strips, machined knobs, plastic fader caps, 24 px M/S keys, warm meters with PEAK, VOICES n / ∞ (studio) or the hardware channel (locked), Main at the right |
| `knobs.*` | **Option A only** (Machined) | B (Soft-touch) and C (Arc) are **superseded** |
| `meter-colours.*` | **Option 2 only** (Warm, punchier amber and red) | 1 (Warm, today) and 3 (Classic LED) are **superseded**; a classic theme is parked in IDEAS.md |
| `workflow-chip-synth.*` | **Locked for layout** | The browser (Collections / Library / Places, folders), the device chain (MIDI effects → Chip Synth → audio effects), the preset field with ◀ ▶, and the same device on a locked track (dimmed, the real channel's controls only) |
| `fixups.*` | **Locked** | 4b's visual fixups, e.g. track numbers ≥ 10 |
| `main-track.*` | **Locked** | The Main row in the arrangement (Batch 4a built it from text; RUNLOG B4A-D1, D15) |
| `cartwave-icon.*` | **Option A locked** ("Chip, refined") | B and C are superseded. A has a simplified variant for 32 px and below |
| `glintworks-logo.*` | **Option A locked** (glint + wordmark + STUDIO) | B and C are superseded |

Notes on reading the boards:

- **The glint has eight points**, not four. Board A's caption says "the soft four-point glint",
  but its drawing has four long curved points and four short diagonal ones, which is what the
  CEO decided (VISION.md "Company, product and business model"). The drawing and the decision
  win over the caption (RUNLOG DOC-D4). Sam's final-file tweaks are not in the board yet.
- **Track rows on `main-track` and `fixups`** are context. They still show the earlier header
  meter gradient and unlocked-looking controls on locked tracks; VISUAL-SPEC.md's rules (header
  meters in the pan grey, "locked means dimmed") win where they differ (DOC-D5).
- The boards name the output device "MOTU M2" as sample data only; it isn't product text.
