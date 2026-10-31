#!/usr/bin/env bash
# Full pre-push gate, mirrors CI plus fmt. Usage: guard.sh [repo_dir]
set -uo pipefail
R=${1:-/home/user/flowmaid}; cd "$R" || exit 2
fail=0
step(){ echo "== $1"; shift; if ! "$@" > /tmp/guard.$$.log 2>&1; then echo "FAILED: $*"; tail -60 /tmp/guard.$$.log; fail=1; else tail -3 /tmp/guard.$$.log | grep -E "test result|warning|error|Finished" | tail -2; fi; }
step fmt cargo fmt --check
step clippy cargo clippy --all-targets -- -D warnings
step test cargo test --all-targets
step doctest cargo test --doc
step msrv cargo +1.75 check --all-targets
step wasm cargo check --lib --target wasm32-unknown-unknown
rm -f /tmp/guard.$$.log
[ $fail = 0 ] && echo "GUARD: ALL GREEN" || echo "GUARD: FAILURES"
exit $fail
