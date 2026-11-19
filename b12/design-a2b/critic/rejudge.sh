#!/usr/bin/env bash
# rejudge.sh <svgdir> <label>: recompute metrics from the SVGs with the live and the corrected (bends study)
# blockmetrics, judge each with its own tables (live: B/metrics; corrected: bends/data fix-* + thresholds-corrected).
set -uo pipefail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
C=$B/design-a2b/critic; BD=$B/design-a2b/bends
src=$1; lab=$2; out=$C/j/$lab; mkdir -p $out
svgs=(); for f in $B/local/in/*.mmd; do n=$(basename $f .mmd); [ -f $src/$n.svg ] && svgs+=("$src/$n.svg"); done
$B/metrics/blockmetrics --struct-dir $B/local/struct --label $lab "${svgs[@]}" > $out/live.tsv 2>/dev/null
$BD/fixed/blockmetrics --struct-dir $B/local/struct --label $lab "${svgs[@]}" > $out/fix.tsv 2>/dev/null
for due in A1 A2; do
 $B/metrics/judge $out/live.tsv --baseline $B/metrics/baseline-038.tsv --start $B/metrics/ref-start.tsv --mermaid $B/metrics/ref-mermaid.tsv --thresholds $B/metrics/thresholds.json --struct-dir $B/local/struct --due $due > $out/live-$due.txt 2>&1
 $B/metrics/judge $out/fix.tsv --baseline $BD/data/fix-038.tsv --start $BD/data/fix-start.tsv --mermaid $BD/data/fix-mermaid.tsv --thresholds $BD/thresholds-corrected/thresholds.json --struct-dir $B/local/struct --due $due > $out/fix-$due.txt 2>&1
done
echo "$lab n=${#svgs[@]} live A2 $(grep -c 'MUST FAIL' $out/live-A2.txt) A1 $(grep -c 'MUST FAIL' $out/live-A1.txt) | corrected A2 $(grep -c 'MUST FAIL' $out/fix-A2.txt) A1 $(grep -c 'MUST FAIL' $out/fix-A1.txt)"
