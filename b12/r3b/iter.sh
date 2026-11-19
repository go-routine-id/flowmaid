#!/usr/bin/env bash
# iter.sh <label>: build the working tree's suite-on CLI + quality example (incremental, r3b/target-on),
# run the local cells (fmrun) and the direction copies (qrun) into r3b/runs/<label>, compare with base-157117e.
set -uo pipefail
R=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/r3b
T=$R/../tools-a2b
L=${1:?label}; shift
out=$R/runs/$L; mkdir -p $out/bin
( cd /home/user/flowmaid && RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR=$R/target-on CARGO_BUILD_JOBS=2 \
  cargo build --release -q --example quality --bin flowmaid ) || exit 1
cp $R/target-on/release/flowmaid $R/target-on/release/examples/quality $out/bin/
echo "rustflags=--cfg flowmaid_blocks" > $out/bin/BUILD-INFO
$T/fmrun.sh $out/bin $out > $out/fmrun.log 2>&1
$T/qrun.sh $out/bin $out > $out/qrun.log 2>&1
python3 $T/cells.py $T/base-157117e $out --quiet > $out/cells.txt 2>&1
python3 $T/qcmp.py $T/base-157117e $out --quiet > $out/dirs.txt 2>&1
tail -1 $out/fmrun.log
cat $out/cells.txt | head -40
cat $out/dirs.txt | cut -c1-1500 | head -40
