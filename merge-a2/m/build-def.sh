#!/usr/bin/env bash
# Default-build (no cfg) release binaries: the identity probe and the CLI, for origin/dev and the merged tree.
set -e
M=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a2/m
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
unset RUSTFLAGS
mkdir -p $M/bin-dev $M/bin-mrg
(cd $M/probe-dev && CARGO_TARGET_DIR=$M/tgt-probe-dev nice -n 19 cargo build --release -q) && cp $M/tgt-probe-dev/release/idprobe $M/bin-dev/idprobe
(cd $M/wt-dev && CARGO_TARGET_DIR=$M/tgt-probe-dev nice -n 19 cargo build --release -q --bin flowmaid) && cp $M/tgt-probe-dev/release/flowmaid $M/bin-dev/flowmaid
rm -rf $M/tgt-probe-dev
(cd $M/probe-mrg && CARGO_TARGET_DIR=$M/tgt-probe-mrg nice -n 19 cargo build --release -q) && cp $M/tgt-probe-mrg/release/idprobe $M/bin-mrg/idprobe
(cd /home/user/flowmaid && CARGO_TARGET_DIR=$M/tgt-probe-mrg nice -n 19 cargo build --release -q --bin flowmaid) && cp $M/tgt-probe-mrg/release/flowmaid $M/bin-mrg/flowmaid
rm -rf $M/tgt-probe-mrg
echo built
