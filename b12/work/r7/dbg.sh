#!/usr/bin/env bash
# dbg.sh: copy the working tree to a scratch tree, add the debug prints there, build the debug probe -> $B/target-dbg
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
T=$B/work/r7/dbgtree
rm -rf $T; mkdir -p $T
for x in src examples tests Cargo.toml Cargo.lock; do cp -r /home/user/flowmaid/$x $T/; done
DBGTREE=$T python3 $B/work/r5/dbg_on.py || exit 1
DBGTREE=$T python3 $B/work/r5/dbg_extra.py || exit 1
DBGTREE=$T python3 $B/work/r7/dbg_stage.py || exit 1
for f in $B/work/r7/dbg_more*.py; do [ -f "$f" ] && { DBGTREE=$T python3 $f || exit 1; }; done
python3 $B/work/r8/dbg_r8.py && $B/work/r7/probedbg/build.sh
echo dbg build done
