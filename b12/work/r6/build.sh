#!/usr/bin/env bash
# build.sh: suite-on release build of the working tree (CLI + quality example)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
cd /home/user/flowmaid
RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/target-on CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality --example zz_dump 2>&1 | grep -E "^(warning|error)" -A12 | head -60
