#!/usr/bin/env bash
# runq.sh <label> [sets...] : rebuild harness, measure class sets, compare with pre (3abbfeb)
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad; C=$S/cl3; V=$S/verify/c-leftovers
L=$1; shift; SETS=${@:-lp cfz cfz2 cq2}
(cd $C/h && CARGO_BUILD_JOBS=2 nice -n 19 cargo build --release -q 2>&1 | tail -5)
mkdir -p $C/out/$L
cd $V
for s in $SETS; do
  case $s in
    lp) nice -n 19 $C/h/target/release/classq2 r2/lp/*.mmd > $C/out/$L/lp.tsv; pre=$C/lp-pre.tsv;;
    cfz) nice -n 19 $C/h/target/release/classq2 r2/cfz/*.mmd > $C/out/$L/cfz.tsv; pre=$V/r2/cfz-pre.tsv;;
    cfz2) nice -n 19 $C/h/target/release/classq2 r2/cfz2/*.mmd > $C/out/$L/cfz2.tsv; pre=$V/r2/cfz2-pre.tsv;;
    cq2) xargs -a $C/cq2-files.txt nice -n 19 $C/h/target/release/classq2 | grep -v '^file' > $C/out/$L/cq2.tsv; pre=$V/r2/cq2-pre.tsv;;
  esac
  echo "=== $s ($L vs pre)"; python3 $C/cmp.py $pre $C/out/$L/$s.tsv ${SHOW:-4}
done
