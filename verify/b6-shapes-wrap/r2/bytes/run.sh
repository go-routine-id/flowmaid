#!/usr/bin/env bash
V=$1; NOW=/home/user/flowmaid/target/release/flowmaid; PRE=$2
cd $V/bytes; : > result.tsv
while read -r sum path; do
  hdr=$(grep -v '^\s*%%' "$path" | grep -v '^\s*$' | sed -n '1,40p' | grep -m1 -oE '^(\s*)(flowchart|graph|stateDiagram(-v2)?|classDiagram|erDiagram|sequenceDiagram|pie|journey|timeline|gitGraph|sankey-beta|sankey|mindmap|architecture-beta|architecture|xychart-beta|---)' | tr -d ' ')
  a=$(timeout 20 $PRE "$path" 2>/dev/null | md5sum | cut -c1-12); ra=$?
  b=$(timeout 20 $NOW "$path" 2>/dev/null | md5sum | cut -c1-12)
  if [ "$a" = "$b" ]; then st=same; else st=DIFF; fi
  printf "%s\t%s\t%s\n" "$st" "${hdr:-?}" "$path" >> result.tsv
done < list.txt
echo done
