#!/usr/bin/env bash
# jrun.sh <variant>: render the local inputs with bin/fm-<variant>, judge A2 and A1
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12; L=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/lateral; v=$1
cd $S/local && ./run.sh fm $L/bin/fm-$v $L/runs/$v b12-$v >/dev/null 2>&1
./run.sh judge $L/runs/$v --due A2 > $L/runs/$v/judge-A2.txt 2>&1
./run.sh judge $L/runs/$v --due A1 > $L/runs/$v/judge-A1.txt 2>&1
echo "$v A2 $(grep -c 'MUST FAIL' $L/runs/$v/judge-A2.txt) A1 $(grep -c 'MUST FAIL' $L/runs/$v/judge-A1.txt) lateral $(grep 'MUST FAIL' $L/runs/$v/judge-A2.txt | grep -c lateral_x) lat_sum $(grep 'MUST FAIL' $L/runs/$v/judge-A2.txt | awk '$3==\"lateral_x\"{s+=$4} END{print s+0}')"
