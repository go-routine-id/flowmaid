#!/usr/bin/env bash
# bon.sh: build the suite-on CLI and quality example
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
cd /home/user/flowmaid && CARGO_BUILD_JOBS=3 RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/target-on cargo build --release --bin flowmaid --example quality 2>&1 | grep -E "^(warning|error)|Finished" | head -20
