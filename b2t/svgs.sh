#!/usr/bin/env bash
# svgs.sh <outdir>: render corpus + examples + test fixtures to SVG with the current release build
set -uo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
O=$1; mkdir -p $O
(cd /home/user/flowmaid && cargo build --release -q --bin flowmaid) || exit 1
B=/home/user/flowmaid/target/release/flowmaid
for f in $S/cmp/corpus/*.mmd /home/user/flowmaid/examples/*.mmd /home/user/flowmaid/tests/fixtures/quality/*.mmd; do
  d=$(basename $(dirname $f)); n=$(basename $f .mmd)
  $B $f > $O/$d.$n.svg 2>/dev/null || echo "ERR $d/$n"
done
