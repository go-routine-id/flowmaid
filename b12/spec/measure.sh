#!/usr/bin/env bash
# measure.sh <label> [ENV=VAL ...] : render spec/in with the experiment CLI under the given env, measure.
set -euo pipefail
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
FM=${FM:-$B/target-spec/release/flowmaid}
label=$1; shift
out=$B/spec/out/$label; mkdir -p "$out"
for f in $B/spec/in/*.mmd; do
  env "$@" "$FM" "$f" -o "$out/$(basename "$f" .mmd).svg" 2>"$out/$(basename "$f" .mmd).err" || echo "FAILED $f" >&2
done
$B/metrics/blockmetrics --struct-dir $B/spec/struct --label "$label" "$out"/*.svg > "$out/metrics.tsv" 2>"$out/blockmetrics.err" || true
python3 - "$out/metrics.tsv" <<'PY'
import sys,csv
rows=list(csv.DictReader(open(sys.argv[1]),delimiter='\t'))
cols=['name','W','H','compact_mean','compact_max','span_max','sib_side','sib_stagger','sib_side_linked','xframe_runs','crossings','edge_len','straight_max','lane_gap_min','lbl_off_mean','lbl_struck','lbl_crowd','node_hits','warn']
print('\t'.join(c[:11] for c in cols))
for r in rows:
    print('\t'.join((r.get(c,'?') or '')[:14] for c in cols))
PY
