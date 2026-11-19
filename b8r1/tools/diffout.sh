#!/usr/bin/env bash
# usage: diffout.sh binA binB  -> lists files whose SVG differs across fixtures/examples/corpus
A=$1; B=$2
for f in /home/user/flowmaid/tests/fixtures/*.mmd /home/user/flowmaid/examples/*.mmd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/cmp/corpus/*.mmd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/flowcorpus/*.mmd; do
  [ -f "$f" ] || continue
  a=$($A "$f" 2>/dev/null | md5sum); b=$($B "$f" 2>/dev/null | md5sum)
  [ "$a" != "$b" ] && echo "$f"
done
