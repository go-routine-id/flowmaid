#!/usr/bin/env bash
# detall.sh <bindir> <outdir> : chips off their line (lead ruling 1) over the 95 local inputs and the
# 332 + 32 direction copies, static and routed; tables <outdir>/det-{local,dirs,dirsx}-{st,rt}.txt
set -uo pipefail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
T=$B/tools-a2b; F=$B/a2c/fr1
bin=${1:?bindir}; out=${2:?outdir}; mkdir -p $out
for set in local dirs dirsx; do
  case $set in local) in=$B/local/in;; dirs) in=$T/inputs/dirs;; dirsx) in=$T/inputs/dirsx;; esac
  rm -rf $out/svg-$set; mkdir -p $out/svg-$set
  $bin/quality $in --svg $out/svg-$set > /dev/null 2>&1
  python3 $F/tools/detach.py $out/det-$set-st.txt $out/svg-$set/*.static.svg
  python3 $F/tools/detach.py $out/det-$set-rt.txt $out/svg-$set/*.routed.svg
done
