#!/bin/bash
# usage: cediff.sh <outdir> : class/ER files of the verifier's diff list, pre vs now SVG bytes
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
R5=$S/verify/b3-backedges/r5
O=$1; mkdir -p $O/pre $O/now; : > $O/differ.txt; : > $O/same.txt
(cd /home/user/flowmaid && cargo build --release -q --bin flowmaid)
while read f; do
  first=$(grep -v '^\s*%%' $f | grep -v '^\s*$' | head -1)
  case "$first" in classDiagram*|erDiagram*) ;; *) continue;; esac
  k=$(echo $f | md5sum | head -c 8)_$(basename $f .mmd)
  $R5/pre/target/release/flowmaid $f > $O/pre/$k.svg 2>/dev/null
  /home/user/flowmaid/target/release/flowmaid $f > $O/now/$k.svg 2>/dev/null
  if cmp -s $O/pre/$k.svg $O/now/$k.svg; then echo "$k $f" >> $O/same.txt; else echo "$k $f" >> $O/differ.txt; fi
done < $S/r5chk/real_diff_files.txt
wc -l $O/differ.txt $O/same.txt
