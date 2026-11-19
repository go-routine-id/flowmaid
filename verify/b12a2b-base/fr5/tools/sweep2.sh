#!/usr/bin/env bash
# sweep2.sh <bindir> <indir> <outdir>: CLI per file with a 20 s timeout; quality over the files that finished
set -uo pipefail
bin=$1; in=$2; out=$3
mkdir -p "$out/svg" "$out/qin"
for f in "$in"/*.mmd; do n=$(basename "$f" .mmd)
  timeout 20 "$bin/flowmaid" "$f" -o "$out/svg/$n.svg" >/dev/null 2>"$out/svg/$n.err"; rc=$?
  if [ $rc = 124 ]; then echo "TIMEOUT $n" | tee -a "$out/timeouts.txt"; rm -f "$out/svg/$n.svg"; continue; fi
  [ $rc = 0 ] || echo "FAIL $n rc=$rc"
  grep -qv '^saved: ' "$out/svg/$n.err" || rm -f "$out/svg/$n.err"
  cp "$f" "$out/qin/"
done
timeout 600 "$bin/quality" "$out/qin" > "$out/q.tsv" 2> "$out/q.err" || echo "quality failed"
