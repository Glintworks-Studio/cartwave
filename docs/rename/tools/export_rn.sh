#!/usr/bin/env bash
# The rename's export gate (RENAME_PROMPT.md §1.5, §3.1): the game export run by the baseline
# build (BASE, a checkout of main before the rename) and this one on the same inputs; every WAV
# compared byte for byte, every JSON compared after mapping its one renamed field back
# ("format": "cartwave-loop" -> "chipdaw-loop"), so any other difference still shows.
# usage: bash docs/rename/tools/export_rn.sh <base checkout> <out dir>
set -u
BASE=$(cd "$1" && pwd); OUT=$2
NEW=$(cd "$(dirname "$0")/../../.." && pwd)
CARGO=${CARGO:-~/.cargo/bin/cargo}
mkdir -p "$OUT"
cases=(
  "showcase|--rate 44100 --loop-bars 8 --stems channels --name showcase|docs/songs/showcase.mid docs/songs/hey.wav"
  "studio|--stems channels --name studio|docs/songs/v7/v7_studio_fader_voices.chipdaw"
  "demo|--stems groups --name demo|docs/songs/v7/demo_band.chipdaw"
  "panned|--stems channels --name panned|docs/songs/v7/v7_studio_panned.chipdaw"
  "v2demo|--bits 24 --rate 48000 --stems none --name v2demo|docs/songs/v2/demo_band.chipdaw"
)
echo "| Input | Files | WAVs identical | JSON (after mapping the format field) |"
echo "|---|---|---|---|"
for c in "${cases[@]}"; do
  IFS='|' read -r name opts inputs <<< "$c"
  for side in base new; do
    root=$BASE; [ $side = new ] && root=$NEW
    rm -rf "$OUT/$side/$name"; mkdir -p "$OUT/$side/$name"
    ins=(); for i in $inputs; do ins+=("$NEW/$i"); done
    (cd "$root" && $CARGO run --release -q -p ui --example game_export -- $opts "${ins[@]}" "$OUT/$side/$name" > "$OUT/$side/$name.log" 2>&1)
  done
  n=0; wav_bad=0; json=""
  for f in "$OUT/base/$name"/*; do
    b=$(basename "$f"); n=$((n + 1))
    case "$b" in
      *.wav) cmp -s "$f" "$OUT/new/$name/$b" || { wav_bad=$((wav_bad + 1)); echo "WAV differs: $name/$b" >&2; } ;;
      *.json)
        if sed 's/"cartwave-loop"/"chipdaw-loop"/' "$OUT/new/$name/$b" | cmp -s "$f" -; then
          if cmp -s "$f" "$OUT/new/$name/$b"; then json="identical"; else json="only \`format\` differs"; fi
        else json="**DIFFERS**"; echo "JSON differs: $name/$b" >&2; fi ;;
    esac
  done
  nn=$(ls "$OUT/new/$name" | wc -l)
  [ "$nn" -ne "$n" ] && echo "file count differs: $name $n vs $nn" >&2
  echo "| $name: \`$inputs\` ($opts) | $n | $([ $wav_bad -eq 0 ] && echo "**all identical**" || echo "**$wav_bad differ**") | ${json:-none written} |"
done
