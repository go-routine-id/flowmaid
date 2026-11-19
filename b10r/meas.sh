#!/usr/bin/env bash
# meas.sh <label> <repo> : quality TSVs for corp/extra/st/fc -> out/<label>/<set>.tsv
D=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r
L=$1; R=${2:-/home/user/flowmaid}; mkdir -p $D/out/$L
(cd $R && cargo build --release -q --example quality 2>&1 | tail -3)
for s in ${SETS:-corp extra st fc}; do
  $R/target/release/examples/quality $D/sets/$s > $D/out/$L/$s.tsv 2>/dev/null &
done
wait
