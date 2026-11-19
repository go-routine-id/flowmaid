#!/usr/bin/env bash
# bytecmp.sh <binA> <binB> <listfile> : print the inputs whose SVG differs between two flowmaid binaries
A=$1; B=$2; n=0; d=0
while read -r f; do
  n=$((n+1))
  if ! cmp -s <($A "$f" 2>&1) <($B "$f" 2>&1); then d=$((d+1)); echo "DIFF $f"; fi
done < "$3"
echo "compared $n, differ $d"
