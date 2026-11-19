#!/usr/bin/env bash
# debug build (switch on, debug prints) -> $B/target-dbg ; restores the tree
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
cd /home/user/flowmaid
python3 $B/work/r3/dbg_on.py || exit 1
RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/target-dbg CARGO_INCREMENTAL=0 cargo build --release --bin flowmaid --example quality 2>&1 | grep -E "^(warning|error)" -A8 | head -40
python3 $B/work/r3/dbg_off.py
cargo fmt
git status --short | head
