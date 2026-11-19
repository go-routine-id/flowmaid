#!/usr/bin/env bash
# bytecmp.sh <pre-bin> <now-bin> files... : per diagram kind, count same/different CLI output
PRE=$1; NOW=$2; shift 2
declare -A same diff
for f in "$@"; do
  kind=$(grep -v '^\s*$' "$f" | grep -v '^\s*%%' | grep -v '^---' | head -40 | grep -m1 -oE '^\s*(flowchart|graph|stateDiagram(-v2)?|classDiagram|erDiagram|sequenceDiagram|pie|journey|timeline|gitGraph|sankey(-beta)?|mindmap|architecture(-beta)?|advance[A-Za-z-]*)' | tr -d ' ')
  kind=${kind:-other}
  a=$($PRE "$f" 2>&1 | md5sum); b=$($NOW "$f" 2>&1 | md5sum)
  if [ "$a" = "$b" ]; then same[$kind]=$(( ${same[$kind]:-0} + 1 )); else diff[$kind]=$(( ${diff[$kind]:-0} + 1 )); [ -n "$LIST" ] && echo "DIFF $kind $f"; fi
done
for k in $(echo "${!same[@]} ${!diff[@]}" | tr ' ' '\n' | sort -u); do echo "$k same=${same[$k]:-0} diff=${diff[$k]:-0}"; done
