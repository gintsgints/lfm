#!/usr/bin/env bash
# Compare lfm's content search against ripgrep on one directory.
#
#   scripts/search_bench.sh <root> <query>...
#
# lfm side: examples/search_bench.rs (in-process, index built once, warm).
# rg side: hyperfine, one process per run, warm page cache.
#   app  ~ `rg | head -200`        (panel shows the first 200 hits)
#   full ~ `rg --count-matches`    (every match)
# Both use literal, smart-case matching, as the panel does.
set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "usage: $0 <root> <query>..." >&2
    exit 2
fi
root=$1
shift
iters=${ITERS:-10}

for tool in rg hyperfine; do
    command -v "$tool" >/dev/null || { echo "missing: $tool" >&2; exit 1; }
done

cd "$(dirname "$0")/.."
cargo build --quiet --release --example search_bench

echo "== lfm"
target/release/examples/search_bench "$root" "$@" --iters "$iters"

echo
echo "== rg  $(rg --files "$root" | wc -l | tr -d ' ') files"
for q in "$@"; do
    qs=$(printf '%q' "$q")
    # rg exits 1 on no match; that is a result here, not a failure.
    hits=$( (rg -F -S -- "$q" "$root" || true) | head -200 | wc -l | tr -d " ")
    matches=$( (rg -F -S --count-matches --no-filename -- "$q" "$root" || true) | awk "{s+=\$1} END {print s+0}")
    files=$( (rg -F -S -l -- "$q" "$root" || true) | wc -l | tr -d " ")
    echo "-- $q: app $hits hits, full $matches hits in $files files"
    hyperfine --warmup 2 --runs "$iters" --style basic --shell=bash \
        -n "app  $q" "(rg -F -S -- $qs '$root' || true) | head -200 >/dev/null" \
        -n "full $q" "rg -F -S --count-matches -- $qs '$root' >/dev/null || true" \
        2>&1 | grep -E 'Time|Range|^Benchmark'
done
