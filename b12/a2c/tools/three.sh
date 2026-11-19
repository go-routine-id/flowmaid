#!/usr/bin/env bash
# three.sh <outdir> <after-svg-dir> <name>...   mermaid | before (cand-d99c073) | after
set -euo pipefail
B12=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
O=$1; A=$2; shift 2; mkdir -p $O
for n in "$@"; do
  args=()
  [ -f $B12/local/out/mermaid/$n.mermaid.svg ] && args+=("mermaid=$B12/local/out/mermaid/$n.mermaid.png")
  args+=("before=$B12/tools-a2b/cand-d99c073/local/svg/$n.svg" "after=$A/$n.svg")
  NODE_PATH=$(npm root -g) node $B12/a2c/tools/three.js $O/$n.three.png "${args[@]}"
done
