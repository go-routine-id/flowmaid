#!/usr/bin/env bash
# sync.sh -- copy the repo's working tree to b3r2/cp and add the debug instrumentation + harness modules
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
D=$S/b3r2/cp
rm -rf $D/src $D/tests $D/examples; mkdir -p $D; (cd /home/user/flowmaid && tar cf - --exclude=./target --exclude=./.git .) | (cd $D && tar xf -)
cp $S/verify/b3-backedges/headcp/src/verify_b3.rs $S/b3r2/h/dump_svgs.rs $D/src/
printf '#[cfg(test)] mod verify_b3;\n#[cfg(test)] mod dump_svgs;\n' >> $D/src/lib.rs
cd $D && python3 $S/b3r2/instr2.py
