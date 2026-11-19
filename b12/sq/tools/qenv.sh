#!/usr/bin/env bash
# qenv.sh <label> <bin-run-dir> [ENV=..]...: run check 3 with the given env knobs on existing binaries, compare
set -uo pipefail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
T=$B/tools-a2b; L=$1; src=$2; shift 2
o=$B/sq/f-$L; mkdir -p $o; rm -rf $o/bin; cp -r $src/bin $o/bin
env "$@" $T/qrun.sh $o/bin $o > /dev/null
python3 $T/qcmp.py ${BASE:-$T/cand-8876984} $o --quiet
