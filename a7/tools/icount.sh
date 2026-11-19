#!/bin/bash
# usage: icount.sh bin file...  -> prints instruction count (millions)
B=$1; shift
for f in "$@"; do
  n=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $B $f 2>&1 >/dev/null | grep -o "Collected : [0-9]*" | awk '{print $3}')
  echo "$(basename $f) $((n/1000000))M"
done
