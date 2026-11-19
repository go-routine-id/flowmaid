#!/usr/bin/env bash
# sw.sh <bindir> <outdir>: static suite-on SVGs of the verifier's fr3 sweep (72) and fr3 inputs (in/, ind/)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; V=$S/verify/b12a2b-base/fr3
bin=$1; out=$2; mkdir -p $out/sw $out/vin3
for f in $V/sweep/in/*.mmd; do n=$(basename $f .mmd); $bin/flowmaid $f -o $out/sw/$n.svg >/dev/null 2>&1; done
for f in $V/in/*.mmd $V/ind/*.mmd $S/b12a2b/fr3/in/*.mmd; do [ -f "$f" ] || continue; n=$(basename $f .mmd); $bin/flowmaid $f -o $out/vin3/$n.svg >/dev/null 2>&1; done
echo rendered $(ls $out/sw | wc -l) $(ls $out/vin3 | wc -l)
