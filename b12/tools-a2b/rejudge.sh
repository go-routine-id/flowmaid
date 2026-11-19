#!/usr/bin/env bash
# rejudge.sh <outdir>
#   Check 1, the measuring half: measures the stored renders of a run with the LIVE yardstick
#   (b12/metrics/blockmetrics) and judges them with the LIVE judge, rules and reference tables (b12/local/run.sh
#   judge / judge-routed, i.e. b12/metrics/{judge,thresholds*.json,baseline-038.tsv,ref-*.tsv}).
#   Reads   <outdir>/local/svg/<name>.svg        static renders of the 95 local inputs (fmrun.sh)
#           <outdir>/local/q/<name>.routed.svg   the routed twins (quality --svg, fmrun.sh)
#   Writes  <outdir>/local/metrics.tsv (+ .warn), metrics-routed.tsv (+ .warn)
#           <outdir>/local/judge-A2.txt          --due A2 (the must-fail lines; the A2 count)
#           <outdir>/local/judge-A1.txt          --due A1
#           <outdir>/local/judge-A2-all.txt      --due A2 --all: every rule line, read by cells.py
#           <outdir>/local/judge-routed-all.txt  routed rules (4.3 h, d), --all, read by cells.py
#   Needs no binary: use it to re-judge a stored baseline after the judge changed.
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
B=$(cd "$HERE/.." && pwd); L=$B/local; M=$B/metrics
out=$(cd "${1:?outdir}" && pwd); d=$out/local
[ -d "$d/svg" ] || { echo "rejudge.sh: no $d/svg (run fmrun.sh first)" >&2; exit 2; }
lab=$(basename "$out")
svgs=(); missing=0
for f in $(ls "$L"/in/*.mmd); do
  n=$(basename "$f" .mmd)
  if [ -s "$d/svg/$n.svg" ]; then svgs+=("$d/svg/$n.svg"); else missing=$((missing + 1)); fi
done
"$M/blockmetrics" --struct-dir "$L/struct" --label "$lab" "${svgs[@]}" > "$d/metrics.tsv" 2> "$d/metrics.warn" || true
routed=("$d"/q/*.routed.svg)
if [ -e "${routed[0]}" ]; then
  "$M/blockmetrics" --struct-dir "$L/struct" --label "$lab-routed" "${routed[@]}" > "$d/metrics-routed.tsv" 2> "$d/metrics-routed.warn" || true
else
  : > "$d/metrics-routed.tsv"; echo "rejudge.sh: no routed SVGs in $d/q" >&2
fi
"$L/run.sh" judge "$d" --due A2 > "$d/judge-A2.txt" 2>&1
"$L/run.sh" judge "$d" --due A1 > "$d/judge-A1.txt" 2>&1
"$L/run.sh" judge "$d" --due A2 --all > "$d/judge-A2-all.txt" 2>&1
"$L/run.sh" judge-routed "$d" --all > "$d/judge-routed-all.txt" 2>&1
c() { grep -c '^MUST FAIL' "$1"; }
echo "rejudge.sh $lab: ${#svgs[@]} static SVGs ($missing missing), ${#routed[@]} routed | live judge --due A2 $(c "$d/judge-A2.txt") must-fail, --due A1 $(c "$d/judge-A1.txt"), routed $(c "$d/judge-routed-all.txt")"
