#!/usr/bin/env bash
# jdiff.sh <a> <b>: must-fail lines in local/<a> but not <b> and vice versa
MY=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/squeeze
f() { grep 'MUST FAIL' $MY/local/$1/judge-A2.txt | sed -E 's/[0-9]+\.[0-9]+|[0-9]+//g' | sort; }
g() { grep 'MUST FAIL' $MY/local/$1/judge-A2.txt | sort; }
echo "== only in $1 (by input+metric):"; comm -23 <(g $1 | awk '{print $1,$2,$3,$4}' | sort -u) <(g $2 | awk '{print $1,$2,$3,$4}' | sort -u)
echo "== only in $2:"; comm -13 <(g $1 | awk '{print $1,$2,$3,$4}' | sort -u) <(g $2 | awk '{print $1,$2,$3,$4}' | sort -u)
