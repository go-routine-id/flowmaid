#!/usr/bin/env bash
# bulk.sh <quality-bin> <label>: quality TSVs over bulkst, bulkfc, the 413 valid b3r4/all inputs -> b12a2b/bulk/<fam>-<label>.tsv
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
Q=$1; L=$2
for fam in bulkst:$S/b8r1/bulkst bulkfc:$S/b8r1/bulkfc all:$S/b12/design-a2b/regress/in/all; do
  n=${fam%%:*}; d=${fam#*:}
  $Q $d > $S/b12a2b/bulk/$n-$L.tsv 2> $S/b12a2b/bulk/$n-$L.err
done
echo done $L
