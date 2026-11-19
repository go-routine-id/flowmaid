#!/usr/bin/env bash
# ton.sh <label> [filter]: suite-on cargo test (lib + tests/quality.rs) -> a2i/ton-<label>.txt / .fail
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2i
cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/target-on CARGO_BUILD_JOBS=3 cargo test --release --no-fail-fast --lib --test quality -- --test-threads=4 $2 > $W/ton-$1.txt 2>&1
grep -E "^test .* FAILED$" $W/ton-$1.txt | sed 's/^test //; s/ \.\.\. FAILED//' | sort > $W/ton-$1.fail
grep "test result" $W/ton-$1.txt
