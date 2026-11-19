#!/usr/bin/env bash
# usage: qall.sh <quality-bin> <label>  -> q.<set>.<label>.tsv for gen gen2 fixtures flowcorpus bulkfc0..3
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; cd $S/b8r1
Q=$1; L=$2
for set in gen gen2 bulkfc0 bulkfc1 bulkfc2 bulkfc3; do ( $Q $set > q.$set.$L.tsv 2>/dev/null ) & done
$Q /home/user/flowmaid/tests/fixtures/quality > q.fix.$L.tsv 2>/dev/null
$Q $S/flowcorpus > q.corp.$L.tsv 2>/dev/null
wait
