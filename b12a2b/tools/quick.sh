#!/usr/bin/env bash
# quick.sh <label> [base-outdir]: checks 1 and 3 (cells, dirs) for b12a2b/bin-<label> against the base (default 7122a53)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; T=$S/b12/tools-a2b
L=$1; B=${2:-$T/base-7122a53}; O=$S/b12a2b/q-$L
mkdir -p $O; cp $S/b12a2b/bin-$L/* $O/ 2>/dev/null
$T/fmrun.sh $S/b12a2b/bin-$L $O > $O/fmrun.log 2>&1
$T/qrun.sh $S/b12a2b/bin-$L $O > $O/qrun.log 2>&1
python3 -I $T/cells.py $B $O > $O/cells.txt 2>&1
python3 -I $T/qcmp.py $B $O > $O/qcmp.txt 2>&1
grep -E "^\[|cells:" $O/cells.txt; grep -E "^\[|dirs:" $O/qcmp.txt
