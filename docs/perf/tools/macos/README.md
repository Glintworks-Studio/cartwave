# macOS perf harness (as used for PERF-macos.md)

No hard-coded paths. Scripts take arguments; `bench_mac.sh` also reads `CARTWAVE_REPO`,
`CARTWAVE_PERF_WORK` (default `$TMPDIR/chipdaw-perf`) and `CARTWAVE_APP`. The calling terminal
needs **Accessibility** (synthetic input) permission; Instruments traces need Xcode.

| File | What it does |
|---|---|
| `chipbench.swift` | Launches the app at 1440×900, injects the scenario (Space, F4, sine drag at 125 Hz), measures process CPU via `proc_pid_rusage` (cross-checked with `ps`), P-core share, energy (`ri_energy_nj`), instructions and cycles. Optional `--framelog`, `--trace` (Metal System Trace or `--template "Time Profiler"`), `--pos`. Prints one JSON line. |
| `bench_mac.sh` | Every cap × scenario with `chipbench`; JSON lines to `$WORK/bench_mac.jsonl` |
| `pacing_mac.py` | App-side frame pacing from `CARTWAVE_FRAMELOG` |
| `displayed_surfaces.py` | Display-side pacing from a Metal System Trace: vblanks each composited frame stayed on screen (the validated outside-the-app method) |
| `tp_categories.py` | Time Profiler CPU breakdown in PERF.md's categories, split by P/E core |
| `display_info.swift` | Refresh rate from NSScreen / CGDisplayMode / CVDisplayLink |
| `validation/testwin.swift` | Reference window updating every k-th vblank: validates the display-side method |
| `validation/sckwin.swift` | ScreenCaptureKit window-update counter: **failed** validation, kept as evidence |

Example:

```sh
swiftc -O docs/perf/tools/macos/chipbench.swift -o /tmp/chipbench
/tmp/chipbench --app target/release/cartwave --scenario demo --secs 30 --env CARTWAVE_FPS_CAP=60 \
  --framelog /tmp/fl.csv --trace /tmp/demo.trace
python3 docs/perf/tools/macos/pacing_mac.py /tmp/fl.csv 30
python3 docs/perf/tools/macos/displayed_surfaces.py /tmp/demo.trace --hz 320
```
