#!/bin/bash
# samecheck.sh: compare SVG output of head worktree vs working tree over corpora
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
n=0; d=0
for f in $S/flowcorpus/*.mmd $S/b8r1/bulkst/*.mmd $S/b8r1/bulkfc0/*.mmd $(ls $S/b8r1/bulkcl/*.mmd | head -300) $S/b10r2/r0/*.mmd; do
  a=$(/home/user/flowmaid/target/release/flowmaid $f 2>/dev/null | md5sum); b=$($S/b10r2/head/target/release/flowmaid $f 2>/dev/null | md5sum)
  n=$((n+1)); if [ "$a" != "$b" ]; then d=$((d+1)); echo "DIFF $f"; fi
done | tail -5
echo "checked"
