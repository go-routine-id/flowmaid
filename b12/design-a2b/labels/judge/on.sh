#!/usr/bin/env bash
# on.sh <cli> <label>: local acceptance static rows + judge (copies of the live judge files)
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
D=$S/design-a2b/labels
J=$D/judge
cli=$1; out=$D/out/$2; mkdir -p $out
svgs=()
for f in $S/local/in/*.mmd; do n=$(basename $f .mmd); "$cli" "$f" -o "$out/$n.svg" >/dev/null 2>&1 || { echo "fail $n"; exit 1; }; svgs+=("$out/$n.svg"); done
"$J/blockmetrics" --struct-dir "$J/struct" --label "$2" "${svgs[@]}" > "$out/metrics.tsv" 2> "$out/metrics.warn" || true
for due in A1 A2; do
"$J/judge" "$out/metrics.tsv" --baseline "$J/baseline-038.tsv" --start "$J/ref-start.tsv" --mermaid "$J/ref-mermaid.tsv" --thresholds "$J/thresholds.json" --struct-dir "$J/struct" --due $due > "$out/judge-$due.txt" 2>&1 || true
done
echo "A2 must-fail: $(grep -c 'MUST FAIL' $out/judge-A2.txt)  A1: $(grep -c 'MUST FAIL' $out/judge-A1.txt)"; tail -1 $out/judge-A2.txt
