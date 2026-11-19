#!/usr/bin/env bash
# judge.sh <label> [cli]: local static acceptance of the B12 inputs with a suite-on CLI -> squeeze/local/<label>; prints A2 / A1 must-fail counts
MY=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/squeeze
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
CLI=${2:-$MY/fm-knobs}
O=$MY/local/$1; mkdir -p $O
$B/local/run.sh fm $CLI $O b12-$1 2>&1 | tail -1
$B/local/run.sh judge $O --due A2 > $O/judge-A2.txt 2>&1
$B/local/run.sh judge $O --due A1 > $O/judge-A1.txt 2>&1
echo "$1: A2 $(grep -c 'MUST FAIL' $O/judge-A2.txt) A1 $(grep -c 'MUST FAIL' $O/judge-A1.txt) | $(tail -1 $O/judge-A2.txt)"
