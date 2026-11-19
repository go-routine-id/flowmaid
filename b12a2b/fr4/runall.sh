#!/usr/bin/env bash
# runall.sh <bindir> <label> [sets...]: sweep every set into $F/s-<label>/<set>
F=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12a2b/fr4
bin=$1; lab=$2; shift 2; sets=${*:-repro tw rnd sw sw72 vin}
for s in $sets; do ( $F/sweep.sh $bin $F/sets/$s $F/s-$lab/$s ) & done; wait
