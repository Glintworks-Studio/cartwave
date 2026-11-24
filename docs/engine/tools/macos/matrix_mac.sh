#!/bin/bash
# usage: matrix_mac.sh [TESTS] [BUFFERS] [LONG_SECS] [SHORT_SECS]
#   TESTS    space-separated, default "A B C D G E" (E256: E at a fixed 256 frames)
#   BUFFERS  default "64 128 256 512"
#   LONG_SECS (A, B) default 600; SHORT_SECS (C, D, G) default 120
# env:
#   CHIPDAW_ENGINE_WORK  output dir (default $TMPDIR/chipdaw-engine)
#   CHIPDAW_APP          app binary (default <repo>/target/release/app)
#   CHIPDAW_GAIN         output gain (default 0.3)
#   CHIPDAW_SWITCH_TO    device for test E to switch to and back from
#                        (default: the built-in speakers, matched by "Speakers")
#   CHIPDAW_NO_TAP=1     run without tapmon
#   CHIPDAW_PILOT_HZ     pilot frequency (default 23000: above the band-limited
#                        synthesis, so music can't mask it; ENGINE-macos.md §3)
#
# The macOS counterpart of ../matrix.ps1 (ENGINE.md §7). Each run writes
# <test>_<buffer>.csv (the app's per-second log and summary, see
# crates/ui/src/testing.rs), <test>_<buffer>.tap.csv (tapmon events: pilot
# dropouts and click onsets, host-clock ns) and, for D/E, an action log.
# Summarise with summarize_mac.py. The terminal needs Accessibility (D) and
# System Audio Recording (tapmon) permission. Test E changes the default
# output device and always restores the original one, also on Ctrl-C.
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../../.." && pwd)"
TESTS="${1:-A B C D G E}"
BUFFERS="${2:-64 128 256 512}"
LONG="${3:-600}"
SHORT="${4:-120}"
W="${CHIPDAW_ENGINE_WORK:-${TMPDIR:-/tmp}/chipdaw-engine}"
APP="${CHIPDAW_APP:-$REPO/target/release/app}"
GAIN="${CHIPDAW_GAIN:-0.3}"
PILOT="${CHIPDAW_PILOT_HZ:-23000}"
mkdir -p "$W/bin"
for t in tapmon coreaudio winctl; do
  if [ ! -x "$W/bin/$t" ] || [ "$HERE/$t.swift" -nt "$W/bin/$t" ]; then
    swiftc -O "$HERE/$t.swift" -o "$W/bin/$t" || exit 1
  fi
done
TAPMON="$W/bin/tapmon"; CA="$W/bin/coreaudio"; WINCTL="$W/bin/winctl"
ORIG_DEFAULT="$("$CA" default | cut -f2)"
restore_default() { "$CA" set-default "$ORIG_DEFAULT" >/dev/null 2>&1; }
trap 'restore_default; pkill -x tapmon 2>/dev/null; exit 130' INT TERM

# run NAME SECS ACTION [K=V ...]
run() {
  local name=$1 secs=$2 action=$3; shift 3
  local log="$W/$name.csv" tap=""
  rm -f "$log" "$W/$name.tap.csv" "$W/$name.tap.txt" "$W/$name.actions.txt"
  if [ "${CHIPDAW_NO_TAP:-0}" != 1 ]; then
    "$TAPMON" --secs $((secs + 60)) --device "$ORIG_DEFAULT" --pilot "$PILOT" --events "$W/$name.tap.csv" --quiet \
      > "$W/$name.tap.txt" 2>&1 &
    tap=$!
    for _ in $(seq 1 50); do grep -q "tapping" "$W/$name.tap.txt" 2>/dev/null && break; sleep 0.1; done
    if ! grep -q "tapping" "$W/$name.tap.txt"; then
      echo "$name: tapmon did not start (permission?); continuing without ground truth"
      kill "$tap" 2>/dev/null; tap=""
    fi
  fi
  env CHIPDAW_AUDIOLOG="$log" CHIPDAW_QUIT_AFTER="$secs" CHIPDAW_PILOT=1 CHIPDAW_PILOT_HZ="$PILOT" CHIPDAW_GAIN="$GAIN" \
    CHIPDAW_FPS_CAP=60 "$@" "$APP" >/dev/null 2>&1 &
  local app=$!
  case "$action" in
    churn) sleep 3; "$WINCTL" "$app" churn $((secs - 10)) > "$W/$name.actions.txt" 2>&1 ;;
    switch)
      local to="${CHIPDAW_SWITCH_TO:-Speakers}"
      { sleep 15; "$CA" set-default "$to"; sleep 15; restore_default; echo "restored $ORIG_DEFAULT"; } \
        > "$W/$name.actions.txt" 2>&1 ;;
  esac
  # QUIT_AFTER plus a margin.
  local waited=0
  while kill -0 "$app" 2>/dev/null; do
    sleep 1; waited=$((waited + 1))
    if [ $waited -gt $((secs + 60)) ]; then echo "$name: TIMEOUT"; kill "$app"; break; fi
  done
  if [ -n "$tap" ]; then sleep 1; kill -INT "$tap" 2>/dev/null; wait "$tap" 2>/dev/null; fi
  echo "== $name"
  grep '^#' "$log" | grep -E "summary|callbacks|underruns|overloads|process" || echo "(no summary)"
  [ -n "$tap" ] && grep "tapmon summary" "$W/$name.tap.txt"
}

for t in $TESTS; do
  case $t in
    A) for b in $BUFFERS; do run "A_$b" "$LONG" none CHIPDAW_BUFFER="$b" CHIPDAW_AUTOPLAY=1; done ;;
    B) for b in $BUFFERS; do run "B_$b" "$LONG" none CHIPDAW_BUFFER="$b" CHIPDAW_STRESS=1 CHIPDAW_AUTO_EDIT=1; done ;;
    C) for b in $BUFFERS; do run "C_$b" "$SHORT" none CHIPDAW_BUFFER="$b" CHIPDAW_AUTOPLAY=1 CHIPDAW_UI_STALL=1; done ;;
    D) for b in $BUFFERS; do run "D_$b" "$SHORT" churn CHIPDAW_BUFFER="$b" CHIPDAW_AVSYNC=1; done ;;
    G) for b in $BUFFERS; do run "G_$b" "$SHORT" none CHIPDAW_BUFFER="$b" CHIPDAW_AVSYNC=1; done ;;
    E) run "E" 50 switch CHIPDAW_AUTOPLAY=1 ;;
    E256) run "E_256" 50 switch CHIPDAW_AUTOPLAY=1 CHIPDAW_BUFFER=256 ;;
  esac
done
restore_default
