#!/usr/bin/env bash
# perfrun.sh <bindir> <label>: quality --drag over the flow corpus into S/runs/<label>/perf.tsv, then the
# route_partial lines of fixture_drag_perf_guard into S/runs/<label>/rp.txt. Waits for a quiet box first.
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
b=$(cd "$1" && pwd); L=$2; mkdir -p $S/runs/$L
for i in $(seq 1 20); do n=$(pgrep -c rustc); l=$(cut -d' ' -f1 /proc/loadavg); if [ "$n" = "0" ] && awk "BEGIN{exit !($l < 1.5)}"; then break; fi; timeout 30 tail -f /dev/null; done
echo "start $(date -u +%T) rustc=$(pgrep -c rustc) load=$(cut -d' ' -f1-3 /proc/loadavg)" > $S/runs/$L/box.txt
$b/quality $S/flowcorpus --drag > $S/runs/$L/perf.tsv 2>/dev/null
echo "mid $(date -u +%T) rustc=$(pgrep -c rustc) load=$(cut -d' ' -f1-3 /proc/loadavg)" >> $S/runs/$L/box.txt
if [ -x $b/lib-tests ]; then (cd /home/user/flowmaid && $b/lib-tests --ignored --test-threads=1 fixture_drag_perf_guard) > $S/runs/$L/rp.txt 2>&1; fi
echo "end $(date -u +%T) rustc=$(pgrep -c rustc) load=$(cut -d' ' -f1-3 /proc/loadavg)" >> $S/runs/$L/box.txt
