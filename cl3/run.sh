#!/usr/bin/env bash
# run.sh <label> [sets...] : rebuild harness, measure class sets, compare with pre (3abbfeb)
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/c-leftovers; O=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/cl3/out
L=$1; shift; SETS=${@:-lp cfz cfz2}
(cd $S/cl3/hn && CARGO_BUILD_JOBS=2 nice -n 19 cargo build --release -q 2>&1 | tail -5)
cd $V
for s in $SETS; do
  if [ $s = cq2 ]; then
    [ -f $O/cq2-pre.tsv ] || (xargs -a $S/cl3/cq2-files.txt $S/cl3/hp/target/release/classq2 | grep -v '^file' > $O/cq2-pre.tsv)
    xargs -a $S/cl3/cq2-files.txt nice -n 19 $S/cl3/hn/target/release/classq2 | grep -v '^file' > $O/cq2-$L.tsv
  else
    ls r2/$s/*.mmd | xargs nice -n 19 $S/cl3/hn/target/release/classq2 > $O/$s-$L.tsv 2>/dev/null
  fi
  echo "=== $s (pre -> $L)"; python3 $S/cl3/cmp.py $O/$s-pre.tsv $O/$s-$L.tsv ${SHOW:-4} | cut -c1-${CUT:-330}
done
