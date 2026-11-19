#!/usr/bin/env bash
# rend.sh <bindir> <outdir>: suite-on static SVGs of the 332+32 copies, the 95 local inputs and the verifier's fr2 inputs
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; T=$S/b12/tools-a2b
bin=$1; out=$2; mkdir -p $out/dirs $out/loc $out/vin
for f in $T/inputs/dirs/*.mmd $T/inputs/dirsx/*.mmd; do n=$(basename $f .mmd); $bin/flowmaid $f -o $out/dirs/$n.svg >/dev/null 2>&1; done
for f in $S/b12/local/in/*.mmd; do n=$(basename $f .mmd); $bin/flowmaid $f -o $out/loc/$n.svg >/dev/null 2>&1; done
for f in $S/verify/b12a2b-base/fr2/in/*.mmd $S/b12a2b/fr2c/in/*.mmd; do [ -f "$f" ] || continue; n=$(basename $f .mmd); $bin/flowmaid $f -o $out/vin/$n.svg >/dev/null 2>&1; done
echo rendered $(ls $out/dirs | wc -l) $(ls $out/loc | wc -l) $(ls $out/vin | wc -l)
