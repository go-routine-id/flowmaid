#!/usr/bin/env bash
# bc.sh <label> [only]   build the working tree suite-on into bin-<label> (skip build if BIN=<dir> given), run checks, compare with the package start
set -uo pipefail
A=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/a2c; T=$A/../tools-a2b
L=$1; only=${2:-local,gates,dirs}
if [ -z "${BIN:-}" ]; then
  CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-3} $T/build.sh /home/user/flowmaid $A/target-on $A/bin-$L > $A/build-$L.log 2>&1 || { tail -30 $A/bin-$L/build.log; exit 1; }
  BIN=$A/bin-$L
fi
rm -rf $A/c-$L
$T/run-checks.sh $A/c-$L --bin $BIN --src /home/user/flowmaid --only $only > $A/c-$L.log 2>&1
$T/compare.sh $T/cand-d99c073 $A/c-$L > $A/c-$L.cmp 2>&1
grep -E "^\[|VERDICT|NEWLY|newly" $A/c-$L.cmp | cut -c1-250
