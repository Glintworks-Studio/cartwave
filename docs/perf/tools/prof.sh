#!/bin/bash
# usage: prof.sh <name> [process=cartwave.exe]
# Imports $CARTWAVE_PERF_WORK/etl/<name>.etl with samply, symbolicates via a local
# samply server, writes $CARTWAVE_PERF_WORK/prof/<name>.{json.gz,txt,folded} and
# copies the .svg/.txt/.folded into $CARTWAVE_REPO/docs/perf/.
# Env: CARTWAVE_PERF_WORK (default $TEMP/chipdaw-perf), CARTWAVE_REPO (default: this
# script's repo). Needs samply, inferno-flamegraph and python on PATH.
set -e
export PATH="$USERPROFILE/.cargo/bin:$PATH"
TOOLS="$(cd "$(dirname "$0")" && pwd)"
S="${CARTWAVE_PERF_WORK:-${TEMP:-/tmp}/chipdaw-perf}"
P="${CARTWAVE_REPO:-$(cd "$TOOLS/../../.." && pwd)}"
NAME="$1"; PROC="${2:-cartwave.exe}"
mkdir -p "$S/prof" "$P/docs/perf"
cd "$S/prof"
SYM=(--windows-symbol-server https://msdl.microsoft.com/download/symbols --windows-symbol-cache "$S/symcache"
     --symbol-dir "$P/target/profiling" --symbol-dir "$P/target/profiling/examples" --symbol-dir "$P/target/before/profiling")
samply import "$S/etl/$NAME.etl" -s -n "${SYM[@]}" -o "$NAME.json.gz" > /dev/null 2>&1
# fresh symbol server for this profile
taskkill //F //IM samply.exe > /dev/null 2>&1 || true
samply load "$NAME.json.gz" -n -P 3333 "${SYM[@]}" > "$S/samply_server.log" 2>&1 &
for i in $(seq 1 50); do grep -q "3333%2F" "$S/samply_server.log" 2>/dev/null && break; sleep 0.3; done
TOK=$(grep -o "3333%2F[a-z0-9]*%2Fprofile" "$S/samply_server.log" | head -1 | sed 's/3333%2F//; s/%2Fprofile//')
python -I "$TOOLS/analyze.py" "$NAME.json.gz" "http://127.0.0.1:3333/$TOK" "$NAME" "$PROC" | head -14
# Flamegraph of the process (CPU-weighted, µs).
inferno-flamegraph --title "$NAME ($PROC, CPU-weighted)" --countname "us" --width 1800 < "$NAME.folded" > "$P/docs/perf/$NAME.svg"
cp "$NAME.txt" "$P/docs/perf/$NAME.txt"
cp "$NAME.folded" "$P/docs/perf/$NAME.folded"
taskkill //F //IM samply.exe > /dev/null 2>&1 || true
