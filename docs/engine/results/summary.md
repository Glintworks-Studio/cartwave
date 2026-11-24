| Test | Requested buffer | Run | Granted frames/cb | Underruns (loopback dropouts / starved / late) | Callback cost mean / p99 / max (% of period) | Audio thread k-cycles / callback | UI thread k-cycles / frame | Other |
|---|---|---|---|---|---|---|---|---|
| A. Demo set playing | 128 | 600 s | 512..1126 | **0** / 0 / 0 | 0.224 / 0.6 / 3.3 | 148.0 | 4497.5 | alloc violations 0 |
| A. Demo set playing | 256 | 600 s | 512..1126 | **0** / 0 / 0 | 0.227 / 0.6 / 3.5 | 149.0 | 4460.6 | alloc violations 0 |
| A. Demo set playing | 512 | 600 s | 512..1126 | **0** / 0 / 0 | 0.22 / 0.6 / 3.4 | 145.4 | 4439.5 | alloc violations 0 |
| B. 200 clips + continuous scripted edits | 128 | 600 s | 512..1126 | **0** / 0 / 0 | 0.082 / 0.3 / 2.9 | 87.0 | 12109.7 | 149995 scripted edits, 118239 snapshots applied, edit→tick max 16.64 ms, send→engine max 10.8 ms, dropped 0; alloc violations 0 |
| B. 200 clips + continuous scripted edits | 256 | 600 s | 512..1126 | **0** / 0 / 0 | 0.08 / 0.3 / 2.9 | 86.4 | 12105.0 | 149995 scripted edits, 118239 snapshots applied, edit→tick max 16.64 ms, send→engine max 10.76 ms, dropped 0; alloc violations 0 |
| B. 200 clips + continuous scripted edits | 512 | 600 s | 512..1126 | **0** / 0 / 0 | 0.08 / 0.3 / 3.1 | 85.6 | 12098.9 | 149993 scripted edits, 118239 snapshots applied, edit→tick max 16.64 ms, send→engine max 10.78 ms, dropped 0; alloc violations 0 |
| C. UI stall 250 ms every 2 s | 128 | 120 s | 512..1126 | **0** / 0 / 0 | 0.219 / 0.6 / 3.1 | 142.9 | 15584.8 | 60 UI stalls; alloc violations 0 |
| C. UI stall 250 ms every 2 s | 256 | 120 s | 512..1126 | **0** / 0 / 0 | 0.218 / 0.6 / 3.5 | 143.4 | 15554.6 | 60 UI stalls; alloc violations 0 |
| C. UI stall 250 ms every 2 s | 512 | 120 s | 512..1126 | **0** / 0 / 0 | 0.217 / 0.6 / 3.2 | 142.9 | 15459.1 | 60 UI stalls; alloc violations 0 |
| D. Resize / minimise / restore | 128 | 120 s | 512..1126 | **0** / 0 / 0 | 0.161 / 0.3 / 3.0 | 106.5 | 2510.1 | 75 pairs (75 crossings, 75 clicks); offset ms mean -10.95 sd 0.23 min -11.65 max -10.32; alloc violations 0 |
| D. Resize / minimise / restore | 256 | 120 s | 512..1126 | **0** / 0 / 0 | 0.16 / 0.3 / 2.9 | 106.8 | 2510.8 | 75 pairs (75 crossings, 75 clicks); offset ms mean -10.99 sd 0.23 min -11.80 max -10.19; alloc violations 0 |
| D. Resize / minimise / restore | 512 | 120 s | 512..1126 | **0** / 0 / 0 | 0.161 / 0.3 / 2.9 | 108.0 | 2512.9 | 75 pairs (75 crossings, 75 clicks); offset ms mean -10.97 sd 0.45 min -12.16 max -8.74; alloc violations 0 |
| G. A/V sync | 128 | 120 s | 512..1126 | **0** / 0 / 0 | 0.162 / 0.3 / 3.2 | 108.1 | 1826.6 | 75 pairs (75 crossings, 75 clicks); offset ms mean -10.98 sd 0.11 min -11.28 max -10.72; alloc violations 0 |
| G. A/V sync | 256 | 120 s | 512..1126 | **0** / 0 / 0 | 0.162 / 0.3 / 3.1 | 108.2 | 1804.2 | 75 pairs (75 crossings, 75 clicks); offset ms mean -10.97 sd 0.10 min -11.28 max -10.75; alloc violations 0 |
| G. A/V sync | 512 | 120 s | 512..1126 | **0** / 0 / 0 | 0.162 / 0.3 / 3.2 | 108.7 | 1813.7 | 75 pairs (75 crossings, 75 clicks); offset ms mean -10.97 sd 0.10 min -11.23 max -10.74; alloc violations 0 |
| E. Default output switched away and back | default | 50 s | 480..1126 | **0** / 2 / 2 | 0.215 / 0.5 / 1.2 | 143.2 | 4507.1 | 2 rebuilds, no crash; alloc violations 0 |
