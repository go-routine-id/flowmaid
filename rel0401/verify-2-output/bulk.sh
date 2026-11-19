#!/bin/bash
# bulk.sh <list> <tag>: lite probe, base then head, in chunks; keep only non-translation pairs.
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/verify-2-output
L=$1; T=$2; mkdir -p $V/bulk/$T; cd $V/bulk/$T
split -l 400 -d -a 3 $L chunk.
for c in chunk.*; do
  n=${c#chunk.}
  nice -n 10 $V/ptarget/release/pbase $c b.$n lite 2>/dev/null
  nice -n 10 $V/ptarget/release/phead $c h.$n lite b.$n 2>/dev/null
  python3 -I $V/cmp.py h.$n b.$n --verdicts v.$n.tsv > cmp.$n.txt 2>&1
  python3 -I $V/cmpinfo.py h.$n b.$n > info.$n.txt 2>&1
  cp h.$n/results.tsv res.h.$n.tsv; cp b.$n/results.tsv res.b.$n.tsv
  # drop translation pairs
  awk -F'\t' '$3=="TRANSLATION"{print $1"."$2".svg"}' v.$n.tsv | while read f; do rm -f h.$n/$f b.$n/$f; done
  # drop base-only leftovers? (none expected: same ones removed by head)
  echo "chunk $n done $(date +%T)"
done
