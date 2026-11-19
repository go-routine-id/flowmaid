#!/usr/bin/env bash
# fmrun.sh <bindir> <outdir>
#   Check 1, the rendering half: renders the 95 local inputs (b12/local/in/*.mmd) with <bindir>/flowmaid (static)
#   and <bindir>/quality --svg (the routed twins), then measures and judges them with the LIVE judge (rejudge.sh).
#   Writes <outdir>/local/svg/<name>.svg (+ <name>.err when the CLI printed more than "saved:"), <outdir>/local/q/,
#   <outdir>/local/quality.tsv (+ .err), and everything rejudge.sh writes. Needs no source tree.
#   A CLI failure is reported and the input stays missing: the judge then fails its named rules ("input missing").
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
B=$(cd "$HERE/.." && pwd); L=$B/local
bin=$(cd "${1:?bindir}" && pwd); out=${2:?outdir}
[ -x "$bin/flowmaid" ] && [ -x "$bin/quality" ] || { echo "fmrun.sh: $bin needs flowmaid and quality (build.sh)" >&2; exit 2; }
mkdir -p "$out/local"; out=$(cd "$out" && pwd); d=$out/local
rm -rf "$d/svg" "$d/q"; mkdir -p "$d/svg"
fail=0
for f in $(ls "$L"/in/*.mmd); do
  n=$(basename "$f" .mmd)
  timeout 120 "$bin/flowmaid" "$f" -o "$d/svg/$n.svg" > /dev/null 2> "$d/svg/$n.err" || { echo "fmrun.sh: flowmaid failed on $n (see $d/svg/$n.err)" >&2; rm -f "$d/svg/$n.svg"; fail=1; }
  grep -qv '^saved: ' "$d/svg/$n.err" || rm -f "$d/svg/$n.err"   # the CLI's own "saved: <path>" line is no news
done
timeout 1200 "$bin/quality" "$L/in" --svg "$d/q" > "$d/quality.tsv" 2> "$d/quality.err" || { echo "fmrun.sh: quality --svg failed (see $d/quality.err)" >&2; fail=1; }
"$HERE/rejudge.sh" "$out" || fail=1
exit $fail
