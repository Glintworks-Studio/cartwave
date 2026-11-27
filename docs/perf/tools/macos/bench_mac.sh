#!/bin/bash
# usage: bench_mac.sh [label] [secs=12] [caps="off 60 120"] [scenarios="idle demo 200 drag"]
#
# macOS counterpart of bench.ps1: process CPU (+ P-core share, energy) for every
# scenario x frame-rate cap. One fresh app launch per cell. Results are appended
# as JSON lines to $WORK/bench_mac.jsonl and printed as a table.
#
# Env:  CARTWAVE_REPO        repo root (default: this script's repo)
#       CARTWAVE_PERF_WORK   work dir for the harness binary and logs (default: $TMPDIR/chipdaw-perf)
#       CARTWAVE_APP         app binary (default: $CARTWAVE_REPO/target/release/cartwave)
#       CARTWAVE_BENCH_ARGS  extra chipbench arguments, e.g. "--pos 0,508" or "--trace" setups
set -euo pipefail
export CARTWAVE_DEV=1   # F-02: test hooks and F3-F7 need dev mode
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="${CARTWAVE_REPO:-$(cd "$HERE/../../../.." && pwd)}"
WORK="${CARTWAVE_PERF_WORK:-${TMPDIR:-/tmp}/chipdaw-perf}"
APP="${CARTWAVE_APP:-$REPO/target/release/cartwave}"
LABEL="${1:-run}"; SECS="${2:-12}"; CAPS="${3:-off 60 120}"; SCENARIOS="${4:-idle demo 200 drag}"
mkdir -p "$WORK"
BENCH="$WORK/chipbench"
if [ ! -x "$BENCH" ] || [ "$HERE/chipbench.swift" -nt "$BENCH" ]; then
  swiftc -O "$HERE/chipbench.swift" -o "$BENCH" 2>/dev/null
fi
pkill -x app 2>/dev/null || true
for cap in $CAPS; do
  for sc in $SCENARIOS; do
    # shellcheck disable=SC2086
    line=$("$BENCH" --app "$APP" --scenario "$sc" --secs "$SECS" --env "CARTWAVE_FPS_CAP=$cap" \
      --framelog "$WORK/fl_${LABEL}_${cap}_${sc}.csv" ${CARTWAVE_BENCH_ARGS:-} 2>/dev/null)
    fps=$(python3 "$HERE/pacing_mac.py" "$WORK/fl_${LABEL}_${cap}_${sc}.csv" "$SECS" --json 2>/dev/null \
      | python3 -c 'import json,sys; print(round(json.load(sys.stdin)["fps"], 1))' 2>/dev/null || echo 0)
    echo "{\"label\": \"$LABEL\", \"cap\": \"$cap\", \"fps\": $fps, ${line#\{}" | tee -a "$WORK/bench_mac.jsonl"
  done
done
