#!/usr/bin/env bash
# Gate 1 (BATCH5_PROMPT.md §8.1; Presets batch method): existing projects sound exactly the same. Every project file in
# the repository (the corpus, the test songs, the saved runs: .chipdaw and .cartwave, tracked or
# not) rendered and exported by main (../before-batch5) and by this branch: the render and export
# hashes must match line for line. The game export on 4c's five inputs, every file compared byte
# for byte. Batch 4c's method (B4C-D12).
# usage: bash docs/batch5/tools/identity_b5.sh <base checkout> <out dir OUTSIDE the repo>
# (B5-D5: the out dir holds every game-export WAV, about 1.4 GB; copy only the hash lists and the
# summary into docs/batch5/identity/.)
set -u
[ $# -eq 2 ] || { echo "usage: identity_b5.sh <base checkout> <out dir outside the repo>"; exit 2; }
# OUT made absolute (the steps cd into each checkout).
BASE=$(cd "$1" && pwd); mkdir -p "$2"; OUT=$(cd "$2" && pwd)
case "$OUT" in "$(cd "$(dirname "$0")/../../.." && pwd)"/*) echo "the out dir must be outside the repo (B5-D5)"; exit 2;; esac
NEW=$(cd "$(dirname "$0")/../../.." && pwd)
CARGO=${CARGO:-~/.cargo/bin/cargo}
mkdir -p "$OUT"
# The list: every project file under the repo (not target/), as Windows paths (the binaries
# can't open MSYS /c/... paths: a first run listed 299 "can't open" lines, identical and useless).
WNEW=$(cd "$NEW" && pwd -W)
(cd "$NEW" && find . -path ./target -prune -o \( -name '*.chipdaw' -o -name '*.cartwave' \) -print | sort | sed "s|^\.|$WNEW|") > "$OUT/list.txt"
cp "$NEW/crates/ui/examples/corpus_hashes.rs" "$BASE/crates/ui/examples/corpus_hashes.rs" 2>/dev/null || true
(cd "$BASE" && $CARGO run --release -q -p ui --example corpus_hashes -- "$OUT/list.txt" > "$OUT/corpus-hashes-main.tsv" 2>/dev/null)
(cd "$NEW" && $CARGO run --release -q -p ui --example corpus_hashes -- "$OUT/list.txt" > "$OUT/corpus-hashes-b5.tsv" 2>/dev/null)
n=$(wc -l < "$OUT/list.txt")
# Six columns per build (path, render, export, channels, console = the locked tracks alone,
# the number of studio tracks).
paste "$OUT/corpus-hashes-main.tsv" "$OUT/corpus-hashes-b5.tsv" > "$OUT/corpus-paired.tsv"
same=$(awk -F'	' '$2!="error" && $1==$7 && $2==$8 && $3==$9 && $4==$10 && $5==$11' "$OUT/corpus-paired.tsv" | wc -l)
console=$(awk -F'	' '$2!="error" && $1==$7 && $5==$11' "$OUT/corpus-paired.tsv" | wc -l)
errs=$(grep -c $'	error	' "$OUT/corpus-hashes-b5.tsv")
echo "corpus: $n files, $same identical (render, export and channels), $errs that neither build opens"
echo "corpus: $console with the locked tracks alone identical (every studio track muted)"
# The files that changed, and whether each has a studio track (A3 may change only those).
awk -F'	' '$2!="error" && !($2==$8 && $3==$9 && $4==$10 && $5==$11) {print $1 "\t" $12}' "$OUT/corpus-paired.tsv" > "$OUT/corpus-changed.txt"
nchg=$(wc -l < "$OUT/corpus-changed.txt")
nostudio=$(awk -F'	' '$2==0' "$OUT/corpus-changed.txt" | wc -l)
echo "corpus: $nchg changed, $nostudio of them without a studio track"
diff "$OUT/corpus-hashes-main.tsv" "$OUT/corpus-hashes-b5.tsv" > "$OUT/corpus-diff.txt" && echo "corpus diff: none"
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
  for side in main b5; do
    root=$BASE; [ $side = b5 ] && root=$NEW
    rm -rf "$OUT/$side/$name"; mkdir -p "$OUT/$side/$name"
    ins=(); for i in $inputs; do ins+=("$NEW/$i"); done
    (cd "$root" && $CARGO run --release -q -p ui --example game_export -- $opts "${ins[@]}" "$OUT/$side/$name" > "$OUT/$side/$name.log" 2>&1)
  done
  for f in "$OUT/main/$name"/*; do
    total=$((total + 1))
    cmp -s "$f" "$OUT/b5/$name/$(basename "$f")" || { bad=$((bad + 1)); echo "differs: $name/$(basename "$f")"; }
  done
done
echo "game export: $total files, $((total - bad)) byte-identical"
