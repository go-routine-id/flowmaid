#!/usr/bin/env bash
# on.sh <label> <flowmaid-bin>: suite-on static SVGs of in-on.txt into son-<label>/<basename-with-dir-hash>.svg
M=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a2
out=$M/son-$1; mkdir -p $out
while read -r f; do
  d=$(basename $(dirname $f)); b=$(basename $f .mmd)
  timeout 60 $2 "$f" -o $out/${d}__${b}.svg > /dev/null 2>&1 || echo "$f rc=$?" >> $out/ERR
done < $M/in-on.txt
