#!/usr/bin/env bash
# Render the 25-diagram corpus with the current repo build and make
# 3-way side-by-sides: mermaid | baseline | this run.
# Usage: render.sh <label> [repo_dir]     -> $S/runs/<label>/
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; R=${2:-/home/user/flowmaid}
( cd "$R" && cargo build --release -q --bin flowmaid )
mkdir -p $S/runs/$L
NODE_PATH=$(npm root -g) node $S/tools/render3.js $S/cmp/corpus $S/runs/$L "$R/target/release/flowmaid" $S/cmp/out $S/runs/baseline
echo "rendered -> $S/runs/$L  (files: <name>.svg, <name>.png, <name>.three.png)"
