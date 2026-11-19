#!/bin/bash
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b7r2
cd $S/sd && cargo build --release -q 2>&1 | grep -E "^error" -A5 | head
rm -rf $S/now; mkdir -p $S/now; xargs -a $S/list.txt -n 20 -P 4 $S/sd/target/release/sd $S/now 2>/dev/null
if [ ! -d $S/basedump ]; then mkdir -p $S/basedump; xargs -a $S/list.txt -n 20 -P 4 $S/sdb/target/release/sd $S/basedump 2>/dev/null; fi
for f in $S/now/*.svg; do b=$S/basedump/$(basename $f); cmp -s $f $b || echo "$(basename $f)"; done
