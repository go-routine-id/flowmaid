#!/bin/bash
S=$1; IN=$2; OUT=$3
mkdir -p $OUT/p $OUT/c
same=0; diff=0; fail=0
for f in $IN/*.mmd; do
  b=$(basename $f .mmd)
  timeout 300 $S/flowmaid-parent $f -o $OUT/p/$b.svg 2>/dev/null; rp=$?
  timeout 300 $S/flowmaid-commit $f -o $OUT/c/$b.svg 2>/dev/null; rc=$?
  if [ $rp -ne 0 ] || [ $rc -ne 0 ]; then echo "FAIL $b p=$rp c=$rc"; fail=$((fail+1)); continue; fi
  if cmp -s $OUT/p/$b.svg $OUT/c/$b.svg; then same=$((same+1)); else echo "DIFF $b"; diff=$((diff+1)); fi
done
echo "same=$same diff=$diff fail=$fail"
