#!/bin/bash
# MAC_CATCHUP §3.2: pacing re-checked on today's build with PERF-macos.md's method: chipbench
# + an Instruments Metal System Trace (displayed_surfaces.py, display side) and the app's frame
# log (pacing_mac.py). Default settings (60 fps cap). Every traced run leaves a 3.5–6 GB
# instruments*.ktrace in $TMPDIR: it is deleted after each run, and a run doesn't start below
# 20 GB free.
#
# usage: pacing_traces.sh <out dir> [cases]   cases: d320 drag320 d60 move (default: all)
set -u
REPO="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$1"; CASES="${2:-d320 drag320 d60 move}"
W="${MC_WORK:-${TMPDIR:-/tmp}/chipdaw-mac-catchup}"
B="$W/bin/chipbench"; WC="$W/bin/winctl"
APP="${APP:-$REPO/target/release/app}"
mkdir -p "$OUT"
D="$W/data-pacing"; rm -rf "$D"; mkdir -p "$D"; echo '{"audio_device":"Mac mini Speakers"}' > "$D/settings.json"
TOOLS="$REPO/docs/perf/tools/macos"
free_gb() { df -g / | awk 'NR==2{print $4}'; }
# The traces and Instruments' raw ktrace files are listed at the end for deletion (by path).
cleanup() { :; }
run() {  # name secs pos scenario [extra chipbench args...]
  local name=$1 secs=$2 pos=$3 sc=$4; shift 4
  [ "$(free_gb)" -lt 20 ] && { echo "$name: less than 20 GB free, skipped"; return; }
  local posarg=(); [ "$pos" != "-" ] && posarg=(--pos "$pos")
  "$B" --app "$APP" --scenario "$sc" --secs "$secs" --content-height 920 --park 1300,800 \
    --framelog "$OUT/$name.fl.csv" --trace "$OUT/$name.trace" --env CHIPDAW_DEV=1 --env CHIPDAW_DATA_DIR="$D" \
    ${posarg[@]+"${posarg[@]}"} "$@" > "$OUT/$name.json" 2>/dev/null
  echo "== $name"; cat "$OUT/$name.json"
  python3 "$TOOLS/pacing_mac.py" "$OUT/$name.fl.csv" "$secs" | head -2
  for disp in external-1 external-0; do
    local hz=320; [ "$disp" = external-0 ] && hz=59.953
    python3 "$TOOLS/displayed_surfaces.py" "$OUT/$name.trace" --display $disp --hz $hz 2>&1 | head -3
  done
  cleanup
}
for c in $CASES; do
  case $c in
    d320) run d320 30 - demo ;;
    drag320) run drag320 30 - 200-drag --grab 760,172 --sway 120,64 ;;
    d60) run d60 120 560,-1312 demo ;;
    move)
      # Play on the 320 Hz display, then move the window 320 -> 60 -> 320 -> 60 Hz every ~10 s.
      [ "$(free_gb)" -lt 20 ] && { echo "move: low disk"; continue; }
      "$B" --app "$APP" --scenario demo --secs 45 --content-height 920 --park 1300,800 --framelog "$OUT/move.fl.csv" \
        --trace "$OUT/move.trace" --env CHIPDAW_DEV=1 --env CHIPDAW_DATA_DIR="$D" --env CHIPDAW_AUDIOLOG="$OUT/move.audio.csv" > "$OUT/move.json" 2>/dev/null &
      bp=$!
      sleep 9
      pid=$(pgrep -x app | head -1)
      for xy in "560 -1312" "560 128" "560 -1312" "560 128"; do sleep 10; "$WC" "$pid" move $xy >> "$OUT/move.actions.txt"; done
      wait $bp
      echo "== move"; cat "$OUT/move.json"; cat "$OUT/move.actions.txt"
      python3 "$TOOLS/pacing_mac.py" "$OUT/move.fl.csv" 45 | head -2
      grep "^#" "$OUT/move.audio.csv" | grep -E "summary|underruns|overloads"
      for disp in external-1 external-0; do
        hz=320; [ "$disp" = external-0 ] && hz=59.953
        python3 "$TOOLS/displayed_surfaces.py" "$OUT/move.trace" --display $disp --hz $hz 2>&1 | head -3
      done
      cleanup ;;
  esac
done
echo "delete after reading (they are large):"; ls -d "$OUT"/*.trace "$TMPDIR"/instruments*.ktrace 2>/dev/null
