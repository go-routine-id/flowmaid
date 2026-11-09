#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b8-layering
f=$1; n=$(basename $f .mmd)
timeout 60 $V/pre/target/release/flowmaid $f > $V/r2/bulk/pre/$n.svg 2>/dev/null; echo "$? $n" >> $V/r2/bulk/pre/status.txt
timeout 60 $V/r2/flowmaid-post $f > $V/r2/bulk/post/$n.svg 2>/dev/null; echo "$? $n" >> $V/r2/bulk/post/status.txt
