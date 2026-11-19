#!/usr/bin/env bash
# ton.sh <label>: suite-on cargo test (lib + tests/quality.rs) -> a2f/ton-<label>.txt / .fail
cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/target-on cargo test --release --no-fail-fast --lib --test quality -- --test-threads=4 > /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2f/ton-$1.txt 2>&1
grep -E "^test .* FAILED$" /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2f/ton-$1.txt | sed 's/^test //; s/ \.\.\. FAILED//' | sort > /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2f/ton-$1.fail
grep "test result" /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2f/ton-$1.txt
