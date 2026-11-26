== before: main ef2ea19 (fork 60907aad)
| Display | Scenario | Build | CPU % (median, range) | fps | M instr / frame | P-core share | mW | frames > 20 ms (app) | n |
|---|---|---|---|---|---|---|---|---|---|
| 320 Hz | 200 | main | 11.21 (11.1–11.3) | 60.0 | 12.70 | 0.89 | 283 | 0 | 2 |
| 320 Hz | 200drag | main | 32.22 (32.1–32.4) | 121.8 | 18.00 | 0.96 | 874 | 0 | 2 |
| 320 Hz | demo | main | 9.54 (9.5–9.6) | 60.0 | 10.86 | 0.92 | 246 | 0 | 2 |
| 320 Hz | idle | main | 0.81 (0.8–0.8) | 0.0 | 0.00 | 0.29 | 5 | 0 | 2 |
== after: mac/catchup (fork 9110e982)
| Display | Scenario | Build | CPU % (median, range) | fps | M instr / frame | P-core share | mW | frames > 20 ms (app) | n |
|---|---|---|---|---|---|---|---|---|---|
| 320 Hz | 200 | new | 11.16 (11.1–11.2) | 60.1 | 12.69 | 0.89 | 282 | 0 | 2 |
| 320 Hz | 200drag | new | 20.05 (19.9–20.2) | 59.8 | 23.27 | 0.94 | 538 | 3 | 2 |
| 320 Hz | demo | new | 9.84 (9.8–9.9) | 60.0 | 10.88 | 0.92 | 249 | 0 | 2 |
| 320 Hz | idle | new | 1.05 (1.0–1.1) | 0.0 | 0.00 | 0.46 | 11 | 0 | 2 |
