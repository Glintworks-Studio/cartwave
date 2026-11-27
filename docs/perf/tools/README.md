# Perf harness (as used for PERF.md)

These scripts drove every measurement in `PERF.md` (Windows only; the macOS harness is in
[`macos/`](macos/README.md)). Nothing is hard-coded:

- **Repo root:** defaults to this folder's repo (`docs/perf/tools/../../..`); override with `-Repo`
  (PowerShell) or `CARTWAVE_REPO` (`prof.sh`).
- **Work dir** (logs, ETL traces, captures, symbol cache): `-Work`, else `$env:CARTWAVE_PERF_WORK`,
  else `%TEMP%\chipdaw-perf`. `capture_server.ps1` must use the same work dir as the caller.
- **Display rate** for `pacing_analyze.py`: `CARTWAVE_REFRESH_HZ` (default 59.951).

Each script starts with a usage line. Example: `pwsh docs/perf/tools/bench.ps1 -Label baseline`.

| Script | What it does |
|---|---|
| `bench.ps1` | Launches the app; samples process CPU for 12 s each: idle, demo playing, 200 clips playing, 200-clip drag |
| `pacing.ps1` + `pacing_analyze.py` | 30 s run with `CARTWAVE_FRAMELOG` + ffmpeg `ddagrab` (DXGI desktop duplication, `dup_frames=0`, no encode): submit-interval stats, frames > 20 ms, refreshes without a new frame |
| `capture_server.ps1` | Elevated helper (one UAC prompt) that only runs `xperf -on` / `xperf -stop -d` for kernel ETW sampling |
| `trace_app.ps1` | Runs a scenario under that helper, then `prof.sh` |
| `prof.sh` + `analyze.py` | `samply import` of the ETL, symbolication via a local samply server, CPU-weighted category report, folded stacks, inferno flamegraph |
| `Sampler.cs` | Non-ETW stack sampler (SuspendThread + GetThreadContext + dbghelp `StackWalk64`) — needed because ETW tracing suppressed the NVIDIA make-current spin |
| `input.ps1` | Synthetic mouse/keyboard input helpers (client coordinates) |
| `snap.ps1`, `pixdiff.ps1`, `crop.ps1`, `shot.ps1` | Deterministic screenshot, pixel diff, zoomed crops |
| `dpi2.ps1` | Reports display-path refresh rate / scaling via `QueryDisplayConfig` (used to debunk "320 Hz") |
| `experiments/vblank_clock.rs` | The rejected H1 experiment: swap interval 0 + DWM-aligned high-resolution waitable timer (see PERF.md §5 H1) |
