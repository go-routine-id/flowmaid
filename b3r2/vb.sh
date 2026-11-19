#!/usr/bin/env bash
# vb.sh <tag> <list> [drag]  -- copy the working tree, add the verifier harness, run it.
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
V=$S/verify/b3-backedges
T=$1; L=$2
D=$S/b3r2/cp
mkdir -p $D
bash $S/b3r2/sync.sh
export VB3_LIST=$L VB3_OUT=$S/b3r2/out_$T.txt
[ "${3:-}" = drag ] && export VB3_DRAG=1
cd $D && CARGO_TARGET_DIR=$S/b3r2/target cargo test --release -q --lib verify_b3::verify_b3 -- --exact --nocapture 2>&1 | tail -5
