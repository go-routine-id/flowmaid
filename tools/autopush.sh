#!/usr/bin/env bash
# Push the B12 work while a run is busy, without touching the run's index or work tree.
#  - finished commits on the work branch go to origin as soon as they exist (fast-forward only);
#  - uncommitted work goes to backup/<package>-wip2 as a snapshot commit made with a temporary index.
# usage: autopush.sh [interval_seconds=600] [max_minutes=115]
set -u
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
REPO=/home/user/flowmaid
BR=ccr-ccc6d89a-5cxd7e
BACKUP=${AUTOPUSH_BACKUP:-backup/b12a1-structure-wip2}
INTERVAL=${1:-600}
MAX_MIN=${2:-115}
PIDFILE="$S/tools/autopush.pid"
LOG="$S/tools/autopush.log"

echo $$ > "$PIDFILE"
trap 'rm -f "$PIDFILE"' EXIT
end=$(( $(date +%s) + MAX_MIN * 60 ))
cd "$REPO" || exit 1

while [ "$(date +%s)" -lt "$end" ]; do
  now=$(date -u +%H:%M)
  if [ "$(git symbolic-ref --short -q HEAD)" = "$BR" ]; then
    # 1. finished commits: fast-forward the remote work branch
    git fetch -q origin "$BR" 2>/dev/null
    if [ -n "$(git rev-list "origin/$BR..HEAD" 2>/dev/null)" ] && git merge-base --is-ancestor "origin/$BR" HEAD; then
      if git push -q origin "HEAD:$BR" 2>>"$LOG"; then echo "$now pushed $BR $(git rev-parse --short HEAD)" >> "$LOG"; fi
    fi
    # 2. work in progress: snapshot with a temporary index, never the run's own index
    if [ -n "$(git status --porcelain)" ]; then
      TMPIDX="$S/tools/autopush-index.$$"
      SNAP=$( export GIT_INDEX_FILE="$TMPIDX"; rm -f "$GIT_INDEX_FILE"
              git read-tree HEAD && git add -A . >/dev/null 2>&1 && T=$(git write-tree) && rm -f "$GIT_INDEX_FILE" &&
              git commit-tree "$T" -p HEAD -m "snapshot(b12): uncommitted work at $now UTC (INCOMPLETE, unverified)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JGDFJkgB3ttBJtcTtc6aL5" )
      rm -f "$TMPIDX"
      if [ -n "$SNAP" ]; then
        if git ls-tree -r --name-only "$SNAP" | grep -q -i -E '(^|/)private/|infra-'; then
          echo "$now SKIPPED snapshot: a private or local acceptance file is in the tree" >> "$LOG"
        elif git push -q -f origin "$SNAP:refs/heads/$BACKUP" 2>>"$LOG"; then
          echo "$now snapshot $BACKUP ${SNAP:0:7}" >> "$LOG"
        fi
      fi
    fi
  else
    echo "$now HEAD is not on $BR; nothing pushed" >> "$LOG"
  fi
  sleep "$INTERVAL"
done
echo "$(date -u +%H:%M) autopush ended after $MAX_MIN min" >> "$LOG"
