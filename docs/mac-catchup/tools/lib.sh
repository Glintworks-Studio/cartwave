# Shared helpers for the MAC_CATCHUP checks (source this file).
#   MC_WORK   work dir (default $TMPDIR/chipdaw-mac-catchup); MC_BIN dir of compiled tools
#   launch <name> [project] [env K=V ...]  -> starts the app with its own data dir; sets $PID, $LOG, $DATA
#   drv <macdrive args...>                 -> macdrive on $PID
#   quit                                   -> kills the app
REPO="${REPO:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"
MC_WORK="${MC_WORK:-${TMPDIR:-/tmp}/chipdaw-mac-catchup}"
MC_BIN="${MC_BIN:-$MC_WORK/bin}"
mkdir -p "$MC_WORK" "$MC_BIN"
for t in macdrive; do
  if [ ! -x "$MC_BIN/$t" ] || [ "$REPO/docs/mac-catchup/tools/$t.swift" -nt "$MC_BIN/$t" ]; then
    swiftc -O "$REPO/docs/mac-catchup/tools/$t.swift" -o "$MC_BIN/$t" 2>/dev/null
  fi
done
APP="${APP:-$REPO/target/release/app}"
launch() {
  local name="$1"; shift
  local proj="${1:-}"; [ $# -gt 0 ] && shift
  pkill -x app 2>/dev/null; sleep 0.5
  DATA="$MC_WORK/data-$name"; rm -rf "$DATA"; mkdir -p "$DATA"
  # Audio: the built-in output unless MC_DEVICE says otherwise (never the system default).
  printf '{"audio_device": "%s"}\n' "${MC_DEVICE:-Mac mini Speakers}" > "$DATA/settings.json"
  LOG="$MC_WORK/$name.log"
  local envs=(CHIPDAW_DEV=1 CHIPDAW_DATA_DIR="$DATA")
  if [ -n "$proj" ]; then cp "$proj" "$MC_WORK/$name.chipdaw"; PROJ="$MC_WORK/$name.chipdaw"; envs+=(CHIPDAW_PROJECT="$PROJ"); fi
  env "${envs[@]}" "$@" "$APP" ${MC_APP_ARGS:-} > "$LOG" 2>&1 &
  PID=$!
  for _ in $(seq 1 50); do "$MC_BIN/macdrive" $PID info 2>/dev/null | grep -q '^window' && break; sleep 0.2; done
  sleep 1.5
  "$MC_BIN/macdrive" $PID focus
}
drv() { "$MC_BIN/macdrive" $PID "$@"; }
quit() { kill $PID 2>/dev/null; sleep 0.5; kill -9 $PID 2>/dev/null; }
clips() { python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print(len(d['clips']))" "$1"; }
