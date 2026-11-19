#!/usr/bin/env bash
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/r7/probedbg
RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/target-dbg CARGO_INCREMENTAL=0 cargo build --release --bins 2>&1 | grep -E "^(warning|error)" -A8 | head -40
