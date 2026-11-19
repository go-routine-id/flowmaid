#!/usr/bin/env bash
# qx.sh <label> <FMX>: quick checks (cells, dirs) of bin-fx with FMX switches, into q-fx-<label>
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; T=$S/b12/tools-a2b
L=$1; export FMX=$2; B=$T/base-7122a53; O=$S/b12a2b/fr3/q-$L
mkdir -p $O; cp $S/b12a2b/bin-fx/* $O/ 2>/dev/null
$T/fmrun.sh $S/b12a2b/bin-fx $O > $O/fmrun.log 2>&1
$T/qrun.sh $S/b12a2b/bin-fx $O > $O/qrun.log 2>&1
python3 -I $T/cells.py $B $O > $O/cells.txt 2>&1
python3 -I $T/qcmp.py $B $O > $O/qcmp.txt 2>&1
echo "== $L FMX=$FMX"; grep -E "^\[local static|cells:" $O/cells.txt; grep -E "^\[dirs static|dirs:|WORSE" $O/qcmp.txt
