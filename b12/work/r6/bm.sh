#!/usr/bin/env bash
# bm.sh <outdir> <cli> <files.mmd...>: render with <cli>, block-metrics rows (xframe_runs etc.)
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
out=$1; cli=$2; shift 2; mkdir -p $out/st
svgs=()
for f in "$@"; do n=$(basename $f .mmd); [ -s $out/st/$n.struct.json ] || $B/start/structdump $f > $out/st/$n.struct.json; $cli $f -o $out/$n.svg 2>/dev/null; svgs+=($out/$n.svg); done
$B/metrics/blockmetrics --struct-dir $out/st --label x "${svgs[@]}" > $out/m.tsv 2> $out/m.warn
python3 - $out/m.tsv <<'PY'
import csv,sys
rows=list(csv.DictReader(open(sys.argv[1]),delimiter='\t'))
cols=['crossings','xframe_runs','fan_x','lateral_x','own_frame_x' ]
cols=[c for c in cols if rows and c in rows[0]]
print('%-30s'%'name',' '.join('%10s'%c[:10] for c in cols))
for r in rows: print('%-30s'%r['name'][:30],' '.join('%10s'%r[c] for c in cols))
PY
