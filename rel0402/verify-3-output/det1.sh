#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-3-output
f="$1"
a=$(timeout 300 $V/probe/new/target/release/newprobe det "$f" 2>&1; echo "rc=$?")
b=$(timeout 300 $V/probe/new/target/release/newprobe det "$f" 2>&1; echo "rc=$?")
if [ "$a" = "$b" ]; then s=DET; else s=NONDET; fi
printf '%s\t%s\n' "$s" "$(printf '%s' "$a" | tr '\n' ' ' | cut -c1-600)"
