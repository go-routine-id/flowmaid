#!/usr/bin/env bash
# on.sh <label> [due]: local acceptance static + judge with the current target-on CLI (short output)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
DUE=${2:-A2}
CLI=${CLI:-$B/target-on/release/flowmaid}
cd $B/local && ./run.sh fm $CLI out/$1 b12-$1 2>&1 | tail -1
./run.sh judge out/$1 --due $DUE > out/$1/judge-$DUE.txt 2>&1
grep -c "MUST FAIL" out/$1/judge-$DUE.txt; tail -1 out/$1/judge-$DUE.txt
