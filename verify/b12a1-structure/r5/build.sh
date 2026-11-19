#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5
cd $V/pre && CARGO_TARGET_DIR=$V/t-pre CARGO_INCREMENTAL=0 nice cargo build --release --bin flowmaid --example quality 2>&1 | tail -2
cd /home/user/flowmaid && CARGO_TARGET_DIR=$V/t-off CARGO_INCREMENTAL=0 nice cargo build --release --bin flowmaid --example quality 2>&1 | tail -2
cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$V/t-on CARGO_INCREMENTAL=0 nice cargo build --release --bin flowmaid --example quality 2>&1 | tail -2
echo BUILD-DONE
