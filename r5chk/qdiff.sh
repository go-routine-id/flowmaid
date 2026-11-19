#!/bin/bash
# usage: qdiff.sh <label> : quality tsv + svgs for HEAD build vs working build over the verifier's dirs
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; O=$S/r5chk/qd/$L; mkdir -p $O
DIRS="$S/flowcorpus $S/verify/b3-backedges/gen $S/verify/a3-state/in $S/verify/a3-state/r6/rand $S/verify/b1-arrows/rnd $S/verify/b1-arrows/r4/rnd $S/verify/b3-backedges/in $S/verify/b3-backedges/in2 $S/verify/b3-backedges/in3 $S/verify/b3-backedges/in4 $S/verify/b3-backedges/in5 $S/verify/b3-backedges/in6 $S/verify/b3-backedges/in7 $S/verify/b3-backedges/in8 $S/verify/b3-backedges/xin $S/verify/b3-backedges/r4/in $S/verify/b3-backedges/r4/lp2 $S/verify/b3-backedges/r5/in $S/verify/b3-backedges/r5/in_lp $S/cmp/corpus"
for d in $DIRS; do [ -d $d ] || continue
  n=$(echo $d | sed "s|$S/||; s|/|_|g")
  for v in head now; do
    if [ $v = head ]; then B=$S/r5chk/headtarget/release/examples/quality; else B=/home/user/flowmaid/target/release/examples/quality; fi
    mkdir -p $O/$v/$n
    $B $d --svg $O/$v/$n > $O/$v/$n.tsv 2>/dev/null
  done
done
echo done
