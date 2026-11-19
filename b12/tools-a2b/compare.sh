#!/usr/bin/env bash
# compare.sh <baseline-outdir> <candidate-outdir> [--quiet]
#   The three checks, baseline B against candidate C (both made by run-checks.sh):
#     1 cells.py   per-cell no-worse on the local inputs, LIVE judge, static (--due A2) and routed rules
#     2 panics.py  gate-message diffs of every suite-on test (lib + tests/quality.rs), not only failing names
#     3 qcmp.py    the 332 + 32 direction copies, static and routed, copy by copy
#   Prints each check's report, then one verdict line per check and an overall line; writes the same text to
#   <candidate-outdir>/compare-vs-<basename of baseline>.txt. Exit status 0 only when all three say "no worse".
#   When the live judge changed since a run was made, re-judge it first: rejudge.sh <outdir> (no rebuild).
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
b=$(cd "${1:?baseline outdir}" && pwd); c=$(cd "${2:?candidate outdir}" && pwd); q=${3:-}
rep=$c/compare-vs-$(basename "$b").txt
{
  echo "compare.sh: baseline $b"; echo "            candidate $c"; echo
  echo "== 1. local inputs, per cell (LIVE judge)"; python3 -I "$HERE/cells.py" "$b" "$c" $q; r1=$?; echo
  echo "== 2. suite-on gate messages"; python3 -I "$HERE/panics.py" "$b" "$c" $q; r2=$?; echo
  echo "== 3. direction copies (332 + 32, static and routed)"; python3 -I "$HERE/qcmp.py" "$b" "$c" $q; r3=$?; echo
  v() { [ "$1" = 0 ] && echo "no worse" || echo "WORSE"; }
  echo "VERDICT cells:  $(v $r1)"
  echo "VERDICT panics: $(v $r2)"
  echo "VERDICT dirs:   $(v $r3)"
  if [ $((r1 + r2 + r3)) = 0 ]; then echo "VERDICT: no worse"; else echo "VERDICT: WORSE"; fi
  exit $(( r1 || r2 || r3 ))
} 2>&1 | tee "$rep"
exit "${PIPESTATUS[0]}"
