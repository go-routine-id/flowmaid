#!/usr/bin/env bash
# Static + interactive (route()/drag) quality & timing over the 14 flowchart corpus files.
# Usage: perf.sh <label> [repo_dir]   -> $S/runs/<label>/perf.tsv ; prints totals and compares to baseline
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
L=$1; R=${2:-/home/user/flowmaid}; mkdir -p $S/runs/$L
( cd "$R" && cargo run --release -q --example quality -- $S/flowcorpus --drag ) > $S/runs/$L/perf.tsv 2>/dev/null
python3 $S/tools/perfcmp.py $S/runs/baseline/perf.tsv $S/runs/$L/perf.tsv
