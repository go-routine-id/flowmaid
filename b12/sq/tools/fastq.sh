#!/usr/bin/env bash
# fastq.sh <label> [base-run-dir]: build the quality example + CLI (suite-on) from the main tree, run dirs (check 3) and compare
set -uo pipefail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
T=$B/tools-a2b; L=$1; base=${2:-$T/cand-8876984}
cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$B/sq/target-on CARGO_BUILD_JOBS=3 cargo build --release --example quality --bin flowmaid 2>&1 | grep -E '^(warning|error)' -A5 | head -20
o=$B/sq/f-$L; mkdir -p $o/bin; cp $B/sq/target-on/release/flowmaid $B/sq/target-on/release/examples/quality $o/bin/
$T/qrun.sh $o/bin $o > /dev/null
python3 $T/qcmp.py $base $o --quiet
