# Upstream write-ups (ready to submit; nothing has been submitted)

Everything ChipDAW found in its dependencies that should go upstream, as files the owner can
submit by hand.

| File | For | What |
|---|---|---|
| [`vizia-prs.md`](vizia-prs.md) | github.com/vizia/vizia | 11 PR descriptions, one per fork commit, with the order and rebase notes |
| [`vizia-patches/`](vizia-patches/) | vizia | The 11 commits as `git format-patch` files (fork `jeffreyfrederick/vizia`, branch `chipdaw-perf`, tag `0.4.0` + 11, ending at `159bbe55`) |
| [`cpal-issues.md`](cpal-issues.md) | github.com/RustAudio/cpal | 5 issue write-ups (3 CoreAudio, 2 WASAPI) |

The patches are the fork's commits as they are. Each commit message is its PR description; the
`.md` file adds a summary, the suggested order and conflicts with upstream `main`.

**Before opening the vizia PRs:** rebase onto current `main`. As of 2026-11-20 (9f84c1d):

- #803 (rrect clip intersection) overlaps commit 4: it fixes window ∩ panel clips, but not
  rect ∩ rounded-corner clips. Keep commit 4's per-shape clipping, but rebase it onto #803.
- `draw_shadows` was rewritten around `rrect` upstream, so commit 3 must be redone and re-measured.
- Commits 1, 2, 5, 6, 10 and 11 touch code that is unchanged on `main`.
- Commit 9's Windows half was later built and measured (PIANOROLL.md Part 0). Commit 10 fixes the
  one regression that measurement found. Submit 9 and 10 together.
