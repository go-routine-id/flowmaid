#!/usr/bin/env bash
# tset.sh <bindir> <outdir>: sequential quality runs (timings) over the generated sets
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
V=$S/verify/b12a2c-squeeze/fr2; W=$S/b12/a2c/fr2/sets
bin=$1; out=$2; mkdir -p $out
for s in gen rnd nr nt sw72 tw i3; do
  d=$W/$s; [ $s = gen ] && d=$V/gen/set
  $bin/quality $d > $out/q-$s.tsv 2>/dev/null
done
