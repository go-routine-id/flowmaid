#!/usr/bin/env bash
# fmrun.sh <label> [ENV=val ...]: render the 95 local inputs with the critic stack CLI under the given env, then rejudge
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12; C=$B/design-a2b/critic
lab=$1; shift; out=$C/svg/$lab; mkdir -p $out
for f in $B/local/in/*.mmd; do n=$(basename $f .mmd); env "$@" timeout 120 $C/target/release/flowmaid "$f" -o "$out/$n.svg" >/dev/null 2>&1 || echo "FAIL $n"; done
$C/rejudge.sh $out $lab
