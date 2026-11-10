#!/usr/bin/env bash
# run1.sh <tag> [ENV=VAL ...]  : render the analog with the prototype and measure it
set -euo pipefail
B12=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
tag=$1; shift
in=${IN:-$B12/in/infra-overview.mmd}
name=$(basename "$in" .mmd)
out=$B12/d-hybrid/out
env "$@" $B12/target-d-hybrid/release/flowmaid "$in" -o $out/$name.$tag.svg >/dev/null
(cd $B12 && node shot.js $out/$name.$tag.svg $out/$name.$tag.png)
st=$B12/metrics/struct/$name.struct.json
if [ ! -s "$st" ]; then st=$B12/d-hybrid/struct/$name.struct.json; [ -s "$st" ] || $B12/metrics/bin/structdump-038 "$in" > "$st"; fi
$B12/metrics/blockmetrics --struct $st --label $tag $out/$name.$tag.svg > $out/$name.$tag.tsv
python3 - "$out/$name.$tag.tsv" <<'PY'
import sys,csv
r=list(csv.reader(open(sys.argv[1]),delimiter='\t'))
d=dict(zip(r[0],r[1]))
keys="W H area compact_mean compact_max compact_max_id span_max sib_side sib_stacked sib_stagger sib_side_linked xframe_runs xframe_len node_hits crossings edge_len straight_max lanes_max lane_gap_min lbl_off_mean lbl_off_max lbl_struck lbl_crowd lbl_gap_min frame_overlap warn".split()
print(d['name'],d['engine'],' '.join(f"{k}={d[k]}" for k in keys))
PY
