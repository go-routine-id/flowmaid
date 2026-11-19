#!/usr/bin/env bash
# det3.sh <bindir> <outdir>: chips' distance to their own drawn path (verifier's det.py) over every set:
# local 95, dirs 332, dirsx 32, the verifier's gen 250 and in/in2/it3 inputs, fr2's generated sets.
set -uo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
B=$S/b12; T=$B/tools-a2b; V=$S/verify/b12a2c-squeeze/fr2; W=$B/a2c/fr2/sets
bin=${1:?bindir}; out=${2:?outdir}; mkdir -p $out
declare -A IN=( [local]=$B/local/in [dirs]=$T/inputs/dirs [dirsx]=$T/inputs/dirsx [gen]=$V/gen/set
  [vin]=$V/in [vin2]=$V/in2 [it3]=$V/it3 [rnd]=$W/rnd [tw]=$W/tw [nr]=$W/nr [nt]=$W/nt [sw72]=$W/sw72 [i3]=$W/i3 )
for set in ${SETS:-${!IN[@]}}; do
  ( rm -rf $out/svg-$set; mkdir -p $out/svg-$set
    timeout 900 $bin/quality ${IN[$set]} --svg $out/svg-$set > $out/q-$set.tsv 2>/dev/null
    python3 $V/tools/det.py $out/svg-$set/*.static.svg > $out/det-$set-st.txt
    python3 $V/tools/det.py $out/svg-$set/*.routed.svg > $out/det-$set-rt.txt ) &
done
wait
