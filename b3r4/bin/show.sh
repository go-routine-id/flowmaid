#!/bin/bash
# usage: show.sh <indir> <outdir> : pre static | now static | now routed -> <out>/<name>.cmp.png
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
T=$S/verify/b3-backedges/tools
I=$1; O=$2; mkdir -p $O/pre $O/now
$S/b3r4/tgt_pre/release/examples/quality $I --svg $O/pre > $O/pre.tsv 2>/dev/null
/home/user/flowmaid/target/release/examples/quality $I --svg $O/now > $O/now.tsv 2>/dev/null
NODE_PATH=$(npm root -g) node $T/svg2png.js $O/pre/*.svg $O/now/*.svg
for f in $I/*.mmd; do n=$(basename $f .mmd); python3 $T/side3.py $O/$n.cmp.png $O/pre/$n.static.png $O/now/$n.static.png $O/now/$n.routed.png 2>/dev/null; done
echo done $O
