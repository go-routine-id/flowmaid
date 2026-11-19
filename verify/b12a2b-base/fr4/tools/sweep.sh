#!/usr/bin/env bash
# sweep.sh <bindir> <indir> <outdir>: static SVGs via the CLI and the harness table (static+routed)
set -uo pipefail
bin=$1; in=$2; out=$3
mkdir -p "$out/svg"
for f in "$in"/*.mmd; do n=$(basename "$f" .mmd); "$bin/flowmaid" "$f" -o "$out/svg/$n.svg" >/dev/null 2>"$out/svg/$n.err" || echo "FAIL $n"; grep -qv '^saved: ' "$out/svg/$n.err" || rm -f "$out/svg/$n.err"; done
"$bin/quality" "$in" > "$out/q.tsv" 2> "$out/q.err" || echo "quality failed"
