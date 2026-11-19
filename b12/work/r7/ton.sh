#!/usr/bin/env bash
# ton.sh <cargo test args...>: suite-on tests of the working tree (dev profile, own target dir)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
cd /home/user/flowmaid
RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/target-ont CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 cargo test "$@" 2>&1
