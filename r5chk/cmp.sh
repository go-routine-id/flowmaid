#!/bin/bash
# usage: cmp.sh <outdir> <files.mmd...> : pre-package | verifier head | now (static SVG via the CLI) side by side
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
R5=$S/verify/b3-backedges/r5; T=$S/verify/b3-backedges/tools
O=$1; shift; mkdir -p $O/pre $O/head $O/now
(cd /home/user/flowmaid && cargo build --release -q --bin flowmaid)
for f in "$@"; do n=$(basename $f .mmd)
  $R5/pre/target/release/flowmaid $f > $O/pre/$n.svg 2>/dev/null
  $R5/head/target/release/flowmaid $f > $O/head/$n.svg 2>/dev/null
  /home/user/flowmaid/target/release/flowmaid $f > $O/now/$n.svg 2>/dev/null
done
NODE_PATH=$(npm root -g) node $T/svg2png.js $O/pre/*.svg $O/head/*.svg $O/now/*.svg >/dev/null 2>&1
for f in "$@"; do n=$(basename $f .mmd); python3 $T/side3.py $O/$n.cmp.png $O/pre/$n.png $O/head/$n.png $O/now/$n.png 2>/dev/null; done
