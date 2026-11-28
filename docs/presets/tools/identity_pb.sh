#!/usr/bin/env bash
# Gate 1 (PRESETS_PROMPT.md §8.1): existing projects sound exactly the same. Every project file in
# the repository (the corpus, the test songs, the saved runs: .chipdaw and .cartwave, tracked or
# not) rendered and exported by main (../before-presets) and by this branch: the render and export
# hashes must match line for line. The game export on 4c's five inputs, every file compared byte
# for byte. Batch 4c's method (B4C-D12).
# usage: bash docs/presets/tools/identity_pb.sh <base checkout> <out dir>
set -u
BASE=$(cd "$1" && pwd); OUT=$2
NEW=$(cd "$(dirname "$0")/../../.." && pwd)
CARGO=${CARGO:-~/.cargo/bin/cargo}
mkdir -p "$OUT"
# The list: every project file under the repo (not target/), as Windows paths (the binaries
# can't open MSYS /c/... paths: a first run listed 299 "can't open" lines, identical and useless).
WNEW=$(cd "$NEW" && pwd -W)
(cd "$NEW" && find . -path ./target -prune -o \( -name '*.chipdaw' -o -name '*.cartwave' \) -print | sort | sed "s|^\.|$WNEW|") > "$OUT/list.txt"
cp "$NEW/crates/ui/examples/corpus_hashes.rs" "$BASE/crates/ui/examples/corpus_hashes.rs" 2>/dev/null || true
(cd "$BASE" && $CARGO run --release -q -p ui --example corpus_hashes -- "$OUT/list.txt" > "$OUT/corpus-hashes-main.tsv" 2>/dev/null)
(cd "$NEW" && $CARGO run --release -q -p ui --example corpus_hashes -- "$OUT/list.txt" > "$OUT/corpus-hashes-pb.tsv" 2>/dev/null)
n=$(wc -l < "$OUT/list.txt")
same=$(paste "$OUT/corpus-hashes-main.tsv" "$OUT/corpus-hashes-pb.tsv" | awk -F'	' '$2!="error" && $1==$5 && $2==$6 && $3==$7 && $4==$8' | wc -l)
errs=$(grep -c $'	error	' "$OUT/corpus-hashes-pb.tsv")
echo "corpus: $n files, $same identical (render, export and channels), $errs that neither build opens"
diff "$OUT/corpus-hashes-main.tsv" "$OUT/corpus-hashes-pb.tsv" > "$OUT/corpus-diff.txt" && echo "corpus diff: none"
cases=(
  "showcase|--rate 44100 --loop-bars 8 --stems channels --name showcase|docs/songs/showcase.mid docs/songs/hey.wav"
  "studio|--stems channels --name studio|docs/songs/v7/v7_studio_fader_voices.chipdaw"
  "demo|--stems groups --name demo|docs/songs/v7/demo_band.chipdaw"
  "panned|--stems channels --name panned|docs/songs/v7/v7_studio_panned.chipdaw"
  "v2demo|--bits 24 --rate 48000 --stems none --name v2demo|docs/songs/v2/demo_band.chipdaw"
)
total=0; bad=0
for c in "${cases[@]}"; do
  IFS='|' read -r name opts inputs <<< "$c"
  for side in main pb; do
    root=$BASE; [ $side = pb ] && root=$NEW
    rm -rf "$OUT/$side/$name"; mkdir -p "$OUT/$side/$name"
    ins=(); for i in $inputs; do ins+=("$NEW/$i"); done
    (cd "$root" && $CARGO run --release -q -p ui --example game_export -- $opts "${ins[@]}" "$OUT/$side/$name" > "$OUT/$side/$name.log" 2>&1)
  done
  for f in "$OUT/main/$name"/*; do
    total=$((total + 1))
    cmp -s "$f" "$OUT/pb/$name/$(basename "$f")" || { bad=$((bad + 1)); echo "differs: $name/$(basename "$f")"; }
  done
done
echo "game export: $total files, $((total - bad)) byte-identical"
