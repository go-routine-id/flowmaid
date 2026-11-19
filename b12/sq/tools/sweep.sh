#!/usr/bin/env bash
# sweep.sh <bindir> <outdir>: quality (static + routed) over the verifier's sets
set -uo pipefail
bin=$1; out=$2; mkdir -p $out
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/sq/../../verify/b12a2b-base
for s in rnd:$V/fr4/rnd/in nr:$V/fr5/nr/in nt:$V/fr5/nt/in sw:$V/fr4/sw/in sw72:$V/fr3/sweep/in tw:$V/fr4/tw/in cin2:$V/fr5/cin2; do
  n=${s%%:*}; d=${s#*:}; ( timeout 900 $bin/quality $d > $out/$n.tsv 2>/dev/null ) &
done; wait
