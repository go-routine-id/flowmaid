#!/usr/bin/env bash
# cli.sh <label>: the CLI of bin-<label> over in-cli.txt, default and --compact 600 (and --advance-text for advance_*):
# one line per run in cli-<label>.tsv: index, mode, exit status, md5 of the SVG (or -), md5 of stderr.
M=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a2/m
out=$M/cli-$1.tsv; : > $out; tmp=$M/cli-tmp-$1; mkdir -p $tmp
i=0
run() { rm -f $tmp/o.svg; timeout 120 $M/bin-$1/flowmaid "${@:3}" -o $tmp/o.svg > /dev/null 2> $tmp/e.txt; rc=$?
  h=$( [ -f $tmp/o.svg ] && md5sum < $tmp/o.svg | cut -c1-32 || echo -); echo -e "$i\t$2\t$rc\t$h\t$(md5sum < $tmp/e.txt | cut -c1-32)" >> $out; }
while read -r f; do
  i=$((i+1))
  run $1 def "$f"
  run $1 cmp "$f" --compact 600
  case $(basename "$f") in advance_*) run $1 adv --advance-text "$f";; esac
done < $M/in-cli.txt
rm -rf $tmp
