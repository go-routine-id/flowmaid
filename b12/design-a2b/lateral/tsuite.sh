#!/usr/bin/env bash
# tsuite.sh <label> [ENV=1 ...]: block-mode lib + quality tests with the given switches -> tests-<label>.fail
L=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/lateral; lab=$1; shift
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/lateral/wt && env "$@" /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/lateral/target/release/deps/flowmaid-8bbc7e834b3bf7f9 --test-threads=3 > $L/tests-$lab.txt 2>&1; env "$@" /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/lateral/target/release/deps/quality-4a6de16462bf1692 --test-threads=3 >> $L/tests-$lab.txt 2>&1
grep -E "^test .* FAILED$" $L/tests-$lab.txt | sed 's/^test //; s/ \.\.\. FAILED//' | sort > $L/tests-$lab.fail
grep "test result" $L/tests-$lab.txt | tr '\n' ' '; echo
