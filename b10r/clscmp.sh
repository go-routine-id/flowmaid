#!/usr/bin/env bash
# clscmp.sh <outdir> : render class/ER list with pre and now; report SAME/CL-ONLY/DIFF
D=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; mkdir -p $O/pre $O/now; : > $O/map.txt
for f in $(cat $D/clser.list); do n=$(echo $f | md5sum | cut -c1-8)_$(basename $f .mmd)
  [ -f $O/pre/$n.svg ] || $S/wt/b10pre/target/release/flowmaid $f > $O/pre/$n.svg 2>/dev/null
  /home/user/flowmaid/target/release/flowmaid $f > $O/now/$n.svg 2>/dev/null
  echo "$n $f" >> $O/map.txt
done
python3 -I $S/verify/b10-orthogonal/scripts/norm.py $O
