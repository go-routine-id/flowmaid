#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure
R7=$V/r7
cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$R7/t-on CARGO_INCREMENTAL=0 nice cargo build --release --bin flowmaid --example quality 2>&1 | tail -1
cd /home/user/flowmaid && CARGO_TARGET_DIR=$R7/t-off CARGO_INCREMENTAL=0 nice cargo build --release --bin flowmaid --example quality 2>&1 | tail -1
cd $V/pre && CARGO_TARGET_DIR=$R7/t-pre CARGO_INCREMENTAL=0 nice cargo build --release --bin flowmaid --example quality 2>&1 | tail -1
cd $R7/probe-pre && CARGO_TARGET_DIR=$R7/tp-pre CARGO_INCREMENTAL=0 nice cargo build --release 2>&1 | tail -1
cd $R7/probe-now && CARGO_TARGET_DIR=$R7/tp-now CARGO_INCREMENTAL=0 nice cargo build --release 2>&1 | tail -1
echo BUILD-DONE
