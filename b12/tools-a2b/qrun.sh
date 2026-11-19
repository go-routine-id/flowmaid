#!/usr/bin/env bash
# qrun.sh <bindir> <outdir>
#   Check 3, the running half: <bindir>/quality (examples/quality, suite-on) over the direction copies, static and
#   routed rows:
#     inputs/dirs   332 copies: every flowchart/state fixture of tests/fixtures/{blocks,quality,notes} drawn TD, BT,
#                   LR and RL (made by work/a2i/tools/mkdirs.py from 7122a53's fixtures; frozen here)
#     inputs/dirsx   32 copies: the 8 examples with subgraphs (advanced, demo, fanout, long-labels, lr, shapes,
#                   stress, subgraph) in the four directions
#   Writes <outdir>/dirs/{dirs,dirsx}.tsv (the full harness table, every column) and .err. qcmp.py compares two.
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
bin=$(cd "${1:?bindir}" && pwd); out=${2:?outdir}
[ -x "$bin/quality" ] || { echo "qrun.sh: $bin/quality missing (build.sh)" >&2; exit 2; }
mkdir -p "$out/dirs"; d=$(cd "$out/dirs" && pwd)
rc=0
for fam in dirs dirsx; do
  timeout 1800 "$bin/quality" "$HERE/inputs/$fam" > "$d/$fam.tsv" 2> "$d/$fam.err" || { echo "qrun.sh: quality failed on $fam (see $d/$fam.err)" >&2; rc=1; }
  echo "qrun.sh: $fam $(( $(grep -vc '^file' "$d/$fam.tsv") / 2 - 1 )) copies x static/routed"
done
exit $rc
