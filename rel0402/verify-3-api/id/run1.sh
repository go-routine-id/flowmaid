#!/usr/bin/env bash
# usage: run1.sh <file>  -> prints "<status> <file>" where status is same/diff/errsame/errdiff
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-3-api
f="$1"
a=$(timeout 60 $V/base/target/release/flowmaid "$f" 2>&1 | md5sum | cut -c1-32); ra=${PIPESTATUS[0]}
b=$(timeout 60 $V/head/target/release/flowmaid "$f" 2>&1 | md5sum | cut -c1-32); rb=${PIPESTATUS[0]}
if [ "$a" = "$b" ]; then echo "same $f"; else echo "diff $f"; fi
