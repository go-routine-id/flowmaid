#!/usr/bin/env bash
# sweep.sh <bindir> <indir> <outdir>
set -uo pipefail
bin=$1; in=$2; out=$3
mkdir -p "$out/svg"
for f in "$in"/*.mmd; do n=$(basename "$f" .mmd); "$bin/flowmaid" "$f" -o "$out/svg/$n.svg" >/dev/null 2>/dev/null || echo "FAIL $n"; done
"$bin/quality" "$in" > "$out/q.tsv" 2> "$out/q.err" || echo "quality failed"
