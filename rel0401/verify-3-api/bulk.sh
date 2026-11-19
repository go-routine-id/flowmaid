#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/verify-3-api
cd $V; mkdir -p bulkres; rm -f bulkres/*; split -l 400 -d -a 3 bulk.lst bulkres/chunk.
for c in bulkres/chunk.*; do
  k=${c##*.}
  rm -rf tb th
  nice -n 15 ./ptarget/release/probe-base dump tb $c > bulkres/b.$k.tsv &
  nice -n 15 ./ptarget/release/probe-head dump th $c > bulkres/h.$k.tsv
  wait
  python3 py/cmp.py bulkres/b.$k.tsv bulkres/h.$k.tsv bulkres/d.$k.tsv > bulkres/s.$k.json
  python3 py/trans.py bulkres/d.$k.tsv tb th > bulkres/t.$k.txt 2>&1
  rm -rf tb th
done
echo BULK DONE
