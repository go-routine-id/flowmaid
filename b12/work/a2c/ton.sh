#!/usr/bin/env bash
# ton.sh <label>: suite-on cargo test (lib + tests/quality.rs) -> a2c/ton-<label>.txt, failing list -> a2c/ton-<label>.fail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12; W=$B/work/a2c
cd /home/user/flowmaid && CARGO_BUILD_JOBS=2 RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/target-on cargo test --release --no-fail-fast --lib --test quality -- --test-threads=3 > $W/ton-$1.txt 2>&1
grep -E "^test .* FAILED$" $W/ton-$1.txt | sed 's/^test //; s/ \.\.\. FAILED//' | sort > $W/ton-$1.fail
grep "test result" $W/ton-$1.txt
