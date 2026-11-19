#!/usr/bin/env bash
# ident.sh <binA> <binB> <files...>: prints count same / differ, and differing file names
A=$1; B=$2; shift 2
same=0; diff=0; errs=0
for f in "$@"; do
  a=$(timeout 60 "$A" "$f" 2>&1 | md5sum); b=$(timeout 60 "$B" "$f" 2>&1 | md5sum)
  if [ "$a" = "$b" ]; then same=$((same+1)); else diff=$((diff+1)); echo "DIFF $f"; fi
done
echo "same=$same differ=$diff"
