#!/bin/bash
# Waits until the phase B run starts pushing B11, then drops an untracked marker in the repo so that the
# script's own pre-check for the next package (c-leftovers) reports a dirty tree and the run stops there.
# (User decision 2026-11-11: B12 runs before c-leftovers.) Re-arm after a container restart.
J=/root/.claude/projects/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/subagents/workflows/wf_5d440b00-741/journal.jsonl
until grep -q '"label":"push:b11-hops-comb"' "$J" 2>/dev/null; do sleep 3; done
touch /home/user/flowmaid/.hold-after-b11
echo "HOLD placed at $(date -u +%H:%M:%S) UTC"
