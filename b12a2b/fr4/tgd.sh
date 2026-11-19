#!/usr/bin/env bash
# tgd.sh <labA> <labB> [sets]: title gaps (Chromium) on the SVGs that differ between runs A and B; columns base | A | B
F=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12a2b/fr4
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2b-base/fr4
a=$1; b=$2; shift 2; sets=${*:-repro tw rnd sw sw72 vin}
t=$F/tg-$a-$b; rm -rf $t; mkdir -p $t/base $t/a $t/b
for s in $sets; do for f in $F/s-$b/$s/svg/*.svg; do n=$(basename $f); cmp -s $f $F/s-$a/$s/svg/$n && continue
  cp $f $t/b/$s--$n; cp $F/s-$a/$s/svg/$n $t/a/$s--$n; cp $F/s-base/$s/svg/$n $t/base/$s--$n 2>/dev/null; done; done
echo "changed: $(ls $t/b | wc -l)"
NODE_PATH=$(npm root -g) node $V/tools/tgfast.js $t/b $t/base $t/a $t/b
