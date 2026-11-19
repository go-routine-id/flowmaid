#!/bin/bash
# usage: cmpall.sh list out.tsv  -> per file: path, rc39, rc40, same/diff
B=$1; list=$2; out=$3
: > "$out"
while IFS= read -r f; do
  [ -f "$f" ] || { printf '%s\tMISSING\n' "$f" >> "$out"; continue; }
  a=$(nice "$B/flowmaid039" "$f" 2>/dev/null | md5sum | cut -c1-32; echo "${PIPESTATUS[0]}")
  b=$(nice "$B/flowmaid040" "$f" 2>/dev/null | md5sum | cut -c1-32; echo "${PIPESTATUS[0]}")
  if [ "$a" = "$b" ]; then s=same; else s=diff; fi
  printf '%s\t%s\t%s\t%s\n' "$f" "$(echo $a | cut -d' ' -f2)" "$(echo $b | cut -d' ' -f2)" "$s" >> "$out"
done < "$list"
