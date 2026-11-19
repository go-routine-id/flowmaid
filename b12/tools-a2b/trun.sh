#!/usr/bin/env bash
# trun.sh <bindir> <outdir> [<src-tree>]
#   Check 2, the running half: the suite-on lib tests and tests/quality.rs (the gates quality_gate,
#   defect_baseline_gate, block_fixtures_are_not_drawn_worse, a_one_px_drag_leaves_unrelated_edges_put,
#   channels_keep_edges_apart_and_off_chips, crossing_gaps_do_not_read_as_dashes, ...), run from the binaries
#   build.sh made (<bindir>/lib-tests, <bindir>/quality-tests), --test-threads=4 --show-output, from the source tree
#   they were built from (<src-tree>, default: src= in <bindir>/BUILD-INFO; the binaries read their fixtures there).
#   Writes <outdir>/gates/lib.txt, quality.txt (full logs), failing.txt (sorted failing test names), results.txt.
#   panics.py compares two such directories.
set -uo pipefail
bin=$(cd "${1:?bindir}" && pwd); out=${2:?outdir}
src=${3:-$(sed -n 's/^src=//p' "$bin/BUILD-INFO" 2>/dev/null)}
[ -n "$src" ] && [ -d "$src/tests/fixtures" ] || { echo "trun.sh: source tree '$src' with tests/fixtures not found; pass it as the third argument" >&2; exit 2; }
for t in lib-tests quality-tests; do [ -x "$bin/$t" ] || { echo "trun.sh: $bin/$t missing (build.sh)" >&2; exit 2; }; done
mkdir -p "$out/gates"; g=$(cd "$out/gates" && pwd)
cd "$src" || exit 2
timeout 1500 "$bin/lib-tests" --test-threads=4 --show-output > "$g/lib.txt" 2>&1; rl=$?
timeout 1500 "$bin/quality-tests" --test-threads=4 --show-output > "$g/quality.txt" 2>&1; rq=$?
grep -hE '^test .* \.\.\. FAILED$' "$g/lib.txt" "$g/quality.txt" | sed 's/^test //; s/ \.\.\. FAILED$//; s/ - should panic$//' | sort > "$g/failing.txt"
{ echo "src=$src"; echo "lib exit=$rl $(grep 'test result' "$g/lib.txt" | tail -1)"; echo "quality exit=$rq $(grep 'test result' "$g/quality.txt" | tail -1)"; } > "$g/results.txt"
for r in $rl $rq; do [ "$r" = 124 ] && echo "trun.sh: a test binary timed out" >&2; done
grep -q 'test result' "$g/lib.txt" && grep -q 'test result' "$g/quality.txt" || { echo "trun.sh: a test binary did not finish (see $g)" >&2; exit 1; }
echo "trun.sh: $(wc -l < "$g/failing.txt") failing; $(sed -n 2p "$g/results.txt" | sed 's/.*test result: //'); quality: $(sed -n 3p "$g/results.txt" | sed 's/.*test result: //')"
