#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure
R6=$V/r6
[ -d $V/pre ] || git -C /home/user/flowmaid worktree add --detach $V/pre 3abbfeb7b3fec70dfa9c30366ca56bf222ca19c5
cd /home/user/flowmaid && CARGO_TARGET_DIR=$R6/t-off CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality 2>&1 | tail -1
cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$R6/t-on CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality 2>&1 | tail -1
cd $R6/probe-pre && CARGO_TARGET_DIR=$R6/tp-pre CARGO_INCREMENTAL=0 cargo build --release 2>&1 | tail -1
cd $R6/probe-now && CARGO_TARGET_DIR=$R6/tp-now CARGO_INCREMENTAL=0 cargo build --release 2>&1 | tail -1
echo BUILD-DONE
