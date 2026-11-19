#!/bin/bash
# usage: chk.sh <out.txt> <files.mmd...>  — loop-attachment + contract check on the main repo's current source
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
W=$S/r5chk/tree
mkdir -p $W
rm -rf $W/src $W/tests $W/examples; cp -r /home/user/flowmaid/src /home/user/flowmaid/tests /home/user/flowmaid/examples /home/user/flowmaid/Cargo.toml /home/user/flowmaid/Cargo.lock $W/ 2>/dev/null; [ -d /home/user/flowmaid/benches ] && cp -r /home/user/flowmaid/benches $W/
cp $S/verify/b3-backedges/r5/vr5.rs $W/src/vr5.rs
echo '#[cfg(test)] mod vr5;' >> $W/src/lib.rs; cp $S/r5chk/dump.rs $W/src/dump.rs; echo '#[cfg(test)] mod dump;' >> $W/src/lib.rs
out=$1; shift
files=$(printf "%s:" "$@"); files=${files%:}
cd $W && VR5_OUT=$out VR5_FILES=$files CARGO_TARGET_DIR=$S/r5chk/target cargo test --release --lib vr5_contract -- --exact vr5::vr5_contract >/dev/null 2>&1
grep -c DETACHED $out; grep "TOTAL" $out
