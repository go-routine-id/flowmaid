#!/usr/bin/env bash
# qrun.sh "<env cfg>" ... -> per-config deltas vs base on quality fixtures
Q=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b7
cd /home/user/flowmaid && cargo build --release -q --tests 2>&1 | tail -3
for cfg in "$@"; do
  n=$(echo "x$cfg" | tr -c 'a-zA-Z0-9\n' '_')
  env $cfg timeout 1200 cargo test --release -q --test quality quality_gate -- --nocapture 2>&1 | grep -E "^(file|back-edge|bench-75|dense-lr|dense-td|fanout-frames|wide-labels)\s" > $Q/qtab_$n.tsv
  echo "== $cfg"
  python3 $Q/bin/qcmp.py $Q/qtab_base.tsv $Q/qtab_$n.tsv ${COLS:+$COLS} | grep -v "^back-edge.*\t0\t0\t0\t" 
done
