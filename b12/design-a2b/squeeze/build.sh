#!/usr/bin/env bash
# build.sh: suite-on (cfg flowmaid_blocks) release build of lib tests, tests/quality.rs, CLI, quality example in the study worktree
MY=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/squeeze
cd $MY/wt && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$MY/target CARGO_BUILD_JOBS=2 cargo test --release --no-run --lib --test quality 2>&1 | grep -E "^(warning: unused|error)|Finished|Executable" | head -20
cd $MY/wt && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$MY/target CARGO_BUILD_JOBS=2 cargo build --release --bin flowmaid --example quality 2>&1 | grep -E "^error|Finished" | head -20
