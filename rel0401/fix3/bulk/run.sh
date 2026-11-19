#!/bin/bash
F=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/fix3
cd $F/bulk
nice -n 15 $F/ptarget/release/phead all all.lst lite > head.tsv 2> head.err
nice -n 15 $F/ptarget/release/pbase all all.lst lite > base.tsv 2> base.err
echo "tsv done $(date +%T)"
rm -f chunk.*
split -l 500 -d -a 3 all.lst chunk.
for c in chunk.*; do
  n=${c#chunk.}
  nice -n 15 $F/ptarget/release/pbase dump $c b.$n lite 2>/dev/null
  nice -n 15 $F/ptarget/release/phead dump $c h.$n lite 2>/dev/null
  python3 -I $F/cmp3.py b.$n h.$n > cmp.$n.txt 2>&1
  rm -rf b.$n h.$n
  echo "chunk $n done $(date +%T)"
done
echo ALLDONE
