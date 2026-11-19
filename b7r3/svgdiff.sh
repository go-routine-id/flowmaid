#!/usr/bin/env bash
# svgdiff.sh : compare flowmaid CLI SVG output head worktree vs working tree over corpus/examples/fixtures/qin
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
(cd $S/b7r3/head && cargo build --release -q --bin flowmaid 2>&1 | grep -E "^error")
(cd /home/user/flowmaid && cargo build --release -q --bin flowmaid 2>&1 | grep -E "^error")
H=$S/b7r3/head/target/release/flowmaid; N=/home/user/flowmaid/target/release/flowmaid
ch=0; tot=0
for f in $S/cmp/corpus/*.mmd /home/user/flowmaid/examples/*.mmd /home/user/flowmaid/tests/fixtures/*/*.mmd $S/b7r3/qin/*.mmd; do
  tot=$((tot+1))
  if ! cmp -s <($H $f 2>/dev/null) <($N $f 2>/dev/null); then ch=$((ch+1)); echo "changed: $f"; fi
done
echo "$ch of $tot changed"
