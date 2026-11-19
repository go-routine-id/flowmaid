#!/usr/bin/env bash
# bulk.sh <label> [r1]: quality tables over corpus, fans, bulkfc0, bulkst with the repo build (or the round-1 copy)
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1
if [ "$2" = r1 ]; then Q=$S/b11r2/r1src/target/release/examples/quality; else (cd /home/user/flowmaid && cargo build --release --example quality -q 2>&1 | tail -2); Q=/home/user/flowmaid/target/release/examples/quality; fi
mkdir -p $S/b11r2/q
$Q /home/user/flowmaid/tests/fixtures/quality > $S/b11r2/q/corp_$L.tsv 2>/dev/null &
$Q $S/b8r1/fan > $S/b11r2/q/fan_$L.tsv 2>/dev/null &
$Q $S/b8r1/bulkfc0 > $S/b11r2/q/fc0_$L.tsv 2>/dev/null &
$Q $S/b8r1/bulkst > $S/b11r2/q/st_$L.tsv 2>/dev/null &
wait
