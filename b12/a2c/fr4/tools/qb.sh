#!/usr/bin/env bash
# qb.sh <bindir>: suite-on release build of the CLI and examples/quality from /home/user/flowmaid's working tree
set -euo pipefail
F=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/a2c/fr4
bin=${1:?bindir}; mkdir -p $bin
export RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$F/target CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-3}
( cd /home/user/flowmaid && cargo build --release --bin flowmaid --example quality ) > $bin/build.log 2>&1 || { tail -30 $bin/build.log; exit 1; }
cp $F/target/release/flowmaid $F/target/release/examples/quality $bin/
echo "qb: built $(cd /home/user/flowmaid && git rev-parse --short HEAD)+dirty into $bin"
