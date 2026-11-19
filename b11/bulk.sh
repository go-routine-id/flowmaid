#!/usr/bin/env bash
# bulk.sh <label>: quality totals over the bulk sets with the repo build (label=now) or HEAD (label=head)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
if [ "$1" = head ]; then Q=$S/b11/headsrc/target/release/examples/quality; else Q=/home/user/flowmaid/target/release/examples/quality; fi
for d in b8r1/bulkfc0 b8r1/bulkst; do
  n=$(basename $d)
  $Q $S/$d > $S/b11/bulk_${n}_$1.tsv 2>$S/b11/bulk_${n}_$1.err
done
