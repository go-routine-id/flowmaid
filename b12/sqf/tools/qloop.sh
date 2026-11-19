#!/usr/bin/env bash
# qloop.sh <label> : build suite-on quality example from the working tree, run the direction copies, compare with c-b
set -uo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
L=$1; Q=$S/sqf
( cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$Q/target-on CARGO_BUILD_JOBS=3 cargo build --release -q --example quality --bin flowmaid ) || exit 1
mkdir -p $Q/f-$L/bin; cp $Q/target-on/release/examples/quality $Q/target-on/release/flowmaid $Q/f-$L/bin/
$S/tools-a2b/qrun.sh $Q/f-$L/bin $Q/f-$L >/dev/null 2>&1
python3 $S/tools-a2b/qcmp.py $Q/c-b $Q/f-$L --quiet
