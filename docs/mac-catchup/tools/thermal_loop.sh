#!/bin/bash
# MAC_CATCHUP §6: a 20-minute demo loop at the default settings (60 fps cap, the built-in output),
# with the thermal state every 30 s (ProcessInfo.thermalState + pmset -g therm, no sudo), the
# app's CPU from ps, and the app's own per-second audio/process log.
# usage: thermal_loop.sh <out dir> [secs=1200] [pos=- | x,y]
set -u
REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$1"; SECS="${2:-1200}"; POS="${3:--}"
W="${MC_WORK:-${TMPDIR:-/tmp}/chipdaw-mac-catchup}"
mkdir -p "$OUT"
D="$W/data-thermal"; rm -rf "$D"; mkdir -p "$D"; echo '{"audio_device":"Mac mini Speakers"}' > "$D/settings.json"
pkill -x app; sleep 0.5
CARTWAVE_DEV=1 CARTWAVE_DATA_DIR="$D" CARTWAVE_AUTOPLAY=1 CARTWAVE_QUIT_AFTER="$SECS" CARTWAVE_AUDIOLOG="$OUT/thermal.audio.csv" \
  "$REPO/target/release/cartwave" > "$OUT/thermal.app.log" 2>&1 &
APPPID=$!
sleep 3
[ "$POS" != "-" ] && "$W/bin/winctl" "$APPPID" move ${POS/,/ } > /dev/null
"$W/bin/macdrive" "$APPPID" move 1300 880 > /dev/null 2>&1
echo "t_s,thermal,cpu_pct_ps" > "$OUT/thermal.samples.csv"
t0=$(date +%s)
while kill -0 "$APPPID" 2>/dev/null; do
  t=$(( $(date +%s) - t0 ))
  st=$(swift -e 'import Foundation; print(["nominal","fair","serious","critical"][ProcessInfo.processInfo.thermalState.rawValue])' 2>/dev/null)
  cpu=$(ps -o %cpu= -p "$APPPID" | tr -d ' ')
  echo "$t,$st,$cpu" >> "$OUT/thermal.samples.csv"
  if [ "$st" = serious ] || [ "$st" = critical ]; then echo "$(date) thermal $st: pausing 10 min" >> "$OUT/thermal.events.txt"; kill "$APPPID"; sleep 600; break; fi
  sleep 30
done
wait "$APPPID" 2>/dev/null
grep '^#' "$OUT/thermal.audio.csv" | grep -E "summary|callbacks|underruns|overloads|process|alloc"
cut -d, -f2 "$OUT/thermal.samples.csv" | sort | uniq -c
