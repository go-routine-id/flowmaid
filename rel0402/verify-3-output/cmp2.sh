#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-3-output
f="$1"
a=$(timeout 300 $V/probe/base/target/release/idprobe "$f" 2>/dev/null; echo "rc	$?")
b=$(timeout 300 $V/probe/head/target/release/idprobe "$f" 2>/dev/null; echo "rc	$?")
na=$(printf '%s\n' "$a" | wc -l)
if [ "$a" = "$b" ]; then echo -e "SAME\t$na\t$(printf '%s\n' "$a" | tail -1 | cut -f2)\t$f"; else
 keys=$(diff <(printf '%s\n' "$a" | cut -f2-) <(printf '%s\n' "$b" | cut -f2-) | grep '^[<>]' | awk '{print $2}' | sort -u | tr '\n' ',')
 echo -e "DIFF\t$na\t$keys\t$f"; fi
