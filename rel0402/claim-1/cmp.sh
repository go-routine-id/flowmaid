#!/bin/bash
C=$1; same=0; diff=0
while read f; do
  a=$(timeout 20 $C/bin/fm-base "$f" 2>&1 | sha1sum); b=$(timeout 20 $C/bin/fm-head "$f" 2>&1 | sha1sum)
  if [ "$a" = "$b" ]; then same=$((same+1)); else diff=$((diff+1)); echo "DIFF $f"; fi
done < $C/big.txt
echo "same=$same diff=$diff"
