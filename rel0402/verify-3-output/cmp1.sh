#!/usr/bin/env bash
# cmp1.sh <file> : render with base and head binaries, print status line
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-3-output
f="$1"
a=$(timeout 60 $V/bin/fm-base "$f" 2>&1; echo "rc=$?")
b=$(timeout 60 $V/bin/fm-head "$f" 2>&1; echo "rc=$?")
ha=$(printf '%s' "$a" | md5sum | cut -c1-32); hb=$(printf '%s' "$b" | md5sum | cut -c1-32)
ra=${a##*rc=}; rb=${b##*rc=}
if [ "$ha" = "$hb" ]; then echo -e "SAME\t$ra\t$f"; else echo -e "DIFF\t$ra/$rb\t$f"; fi
