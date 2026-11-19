#!/usr/bin/env bash
C=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/critic
cd $C/wt && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$C/target CARGO_BUILD_JOBS=2 cargo test --release --no-run --lib --test quality 2>&1 | grep -E "^(warning|error)|Finished|Executable" | head -30
cd $C/wt && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$C/target CARGO_BUILD_JOBS=2 cargo build --release --bin flowmaid --example quality 2>&1 | grep -E "^error|Finished" | head -20
