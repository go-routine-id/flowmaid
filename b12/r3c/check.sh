#!/usr/bin/env bash
# check.sh <label>: build the working tree suite-on (incremental, /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3c/target-on) and run the three checks into /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3c/runs/<label>, compare with base-157117e
set -uo pipefail
L=${1:?label}; out=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3c/runs/$L; mkdir -p $out
CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-3} /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/tools-a2b/build.sh /home/user/flowmaid /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3c/target-on $out/bin || exit 1
/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/tools-a2b/run-checks.sh $out --bin $out/bin --src /home/user/flowmaid > $out/run.log 2>&1
/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/tools-a2b/compare.sh /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/tools-a2b/base-157117e $out > $out/compare.txt 2>&1
grep -E '^\[|VERDICT' $out/compare.txt
