#!/usr/bin/env bash
set -x
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/pre && CARGO_TARGET_DIR=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/t-pre CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality 2>&1 | tail -2
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/now && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/t-on CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality 2>&1 | tail -2
cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/prev && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r4/t-prev CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality 2>&1 | tail -2
echo BUILD-DONE
