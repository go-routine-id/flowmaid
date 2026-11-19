#!/usr/bin/env bash
# dbg.sh: copy the working tree to a scratch tree, add the debug prints there, build it with the switch on -> $B/target-dbg
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
T=$B/work/r6/dbgtree
rm -rf $T; mkdir -p $T
for x in src examples tests Cargo.toml Cargo.lock; do cp -r /home/user/flowmaid/$x $T/; done
cp $B/work/r4/dump.rs $T/examples/dump.rs
DBGTREE=$T python3 $B/work/r5/dbg_on.py || exit 1
DBGTREE=$T python3 $B/work/r5/dbg_extra.py || exit 1
cd $T && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/target-dbg CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality --example dump 2>&1 | grep -E "^(warning|error)" -A8 | head -40
echo dbg build done
