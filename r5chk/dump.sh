#!/bin/bash
# usage: dump.sh <file.mmd> <edges csv> : dump scene edges with the main repo's current source
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
W=$S/r5chk/tree; mkdir -p $W
rm -rf $W/src $W/tests $W/examples; cp -r /home/user/flowmaid/src /home/user/flowmaid/tests /home/user/flowmaid/examples /home/user/flowmaid/Cargo.toml /home/user/flowmaid/Cargo.lock $W/
cp $S/verify/b3-backedges/r5/vr5.rs $W/src/vr5.rs; cp $S/r5chk/dump.rs $W/src/dump.rs
echo '#[cfg(test)] mod vr5;' >> $W/src/lib.rs; echo '#[cfg(test)] mod dump;' >> $W/src/lib.rs
cd $W && DUMP_FILE=$1 DUMP_EDGES=$2 DUMP_OUT=$S/r5chk/now_dump.txt CARGO_TARGET_DIR=$S/r5chk/target cargo test --release --lib dump_edges >/dev/null 2>&1
cat $S/r5chk/now_dump.txt
