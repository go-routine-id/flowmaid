#!/usr/bin/env bash
# det.sh <file.mmd> [quality-binary]: the harness's detail lines of one input (static and routed)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
Q=${2:-$B/target-on/release/examples/quality}
n=$(basename "$1" .mmd); d=$B/work/r4/det/$n; mkdir -p "$d"; cp "$1" "$d/$n.mmd"
"$Q" "$d" --details 2>&1 | grep -vE "^(file|TOTAL)|\s(static|routed)\s+[0-9]"
