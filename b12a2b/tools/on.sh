#!/usr/bin/env bash
# on.sh <label>: suite-on release build of the CLI and the quality example from /home/user/flowmaid
# into b12a2b/target-on, binaries copied to b12a2b/bin-<label>
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1
cd /home/user/flowmaid
CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2} RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$S/b12a2b/target-on \
  cargo build --release -q --bin flowmaid --example quality || exit 1
mkdir -p $S/b12a2b/bin-$L
cp $S/b12a2b/target-on/release/flowmaid $S/b12a2b/target-on/release/examples/quality $S/b12a2b/bin-$L/
echo built $L
