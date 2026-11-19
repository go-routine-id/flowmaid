#!/usr/bin/env bash
# ton.sh <label> [filter]: suite-on cargo test (lib + tests/quality.rs) in the study worktree -> squeeze/ton-<label>.txt / .fail
MY=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/squeeze
cd $MY/wt && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$MY/target CARGO_BUILD_JOBS=2 cargo test --release --no-fail-fast --lib --test quality -- --test-threads=2 $2 > $MY/ton-$1.txt 2>&1
grep -E "^test .* FAILED$" $MY/ton-$1.txt | sed 's/^test //; s/ \.\.\. FAILED//' | sort > $MY/ton-$1.fail
grep "test result" $MY/ton-$1.txt
