#!/usr/bin/env bash
# one.sh <quality-bin> <outdir> <file.mmd>: harness rows of one input (static+routed), 30 s cap
q=$1; out=$2; f=$3; n=$(basename $f .mmd)
d=$(mktemp -d -p /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r5/bulk/tmp); ln -s $f $d/$n.mmd
timeout 30 $q $d > $out/$n.tsv 2>/dev/null || echo "TIMEOUT $n" >> $out/timeouts.txt
rm -rf $d
