#!/usr/bin/env bash
# q.sh <label> : suite-on build of the CLI + quality example from /home/user/flowmaid (working tree),
# render the 95 local inputs, judge, compare cells with the package start, measure detach.
set -uo pipefail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
T=$B/tools-a2b; F=$B/a2c/fr1
lab=${1:?label}
export RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$F/target-on CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-3}
( cd /home/user/flowmaid && cargo build --release --bin flowmaid --example quality ) > $F/build-$lab.log 2>&1 || { tail -20 $F/build-$lab.log; exit 1; }
mkdir -p $F/c-$lab/bin
cp $F/target-on/release/flowmaid $F/target-on/release/examples/quality $F/c-$lab/bin/
$T/fmrun.sh $F/c-$lab/bin $F/c-$lab > $F/c-$lab.log 2>&1
tail -1 $F/c-$lab.log
python3 $T/cells.py $T/cand-d99c073 $F/c-$lab > $F/c-$lab.cmp 2>&1
grep -E '^\[|WORSE|NEW |CHANGED|MISSING' $F/c-$lab.cmp | head -40
python3 $F/tools/detach.py $F/c-$lab/detach-static.txt $F/c-$lab/local/svg/*.svg
