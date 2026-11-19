#!/usr/bin/env bash
# rebuild flowmaid release bin + dbg
(cd /home/user/flowmaid && cargo build --release -q 2>&1 | grep -E "^(error|warning)" -A5)
(cd /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r/dbg && cargo build --release -q 2>&1 | grep -E "^(error|warning)" -A5)
true
