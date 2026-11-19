#!/usr/bin/env bash
# ident.sh: default-build identity, 8876984 vs 15b5383: probe hashes + CLI SVG bytes (default, --compact 600, --advance)
M=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a2
cat $M/in-repo.txt $M/in-extra.txt $M/in-sq.txt $M/in-bulk.txt | sort -u > $M/in-all.txt
for c in 8876984 15b5383; do
  xargs -a $M/in-all.txt -n 200 $M/bin-def-$c/idprobe > $M/id-$c.tsv 2>/dev/null
  mkdir -p $M/cli-$c
  i=0
  while read -r f; do
    i=$((i+1))
    for mode in def cmp adv; do
      case $mode in def) a=();; cmp) a=(--compact 600);; adv) a=(--advance);; esac
      timeout 60 $M/bin-def-$c/flowmaid "$f" "${a[@]}" -o $M/cli-$c/$i.$mode.svg > /dev/null 2>&1 || echo "rc=$?" > $M/cli-$c/$i.$mode.err
    done
  done < $M/in-all.txt
done
