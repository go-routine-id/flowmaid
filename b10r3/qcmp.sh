#!/bin/bash
# qcmp.sh <dir> : quality TOTAL rows (static|routed) for HEAD build vs working tree
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
D=$1
(cd $S/b10r3/headsrc && cargo build --release -q --example quality 2>/dev/null)
(cd /home/user/flowmaid && cargo build --release -q --example quality 2>/dev/null)
$S/b10r3/headsrc/target/release/examples/quality $D > $S/b10r3/q_head.tsv 2>/dev/null
/home/user/flowmaid/target/release/examples/quality $D > $S/b10r3/q_now.tsv 2>/dev/null
python3 - "$S/b10r3/q_head.tsv" "$S/b10r3/q_now.tsv" <<'PY'
import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
a=load(sys.argv[1]); b=load(sys.argv[2])
cols=['defects','x_drawn','edge_node_x','edge_frame_pairs','overlap_pairs','tight_pairs','border_runs','lbl_edge','lbl_node','wiggles','end_cross','idiom_scubic','idiom_bow','band_pairs','width']
for m in ['static','routed']:
    ka=('TOTAL',m)
    print(m, ' '.join(f"{c}:{a[ka].get(c,'?')}->{b[ka].get(c,'?')}" for c in cols if a[ka].get(c)!=b[ka].get(c)) or 'same')
diff=[k for k in b if k[0]!='TOTAL' and k in a and any(a[k].get(c)!=b[k].get(c) for c in ['defects','x_drawn','edge_node_x','overlap_pairs','lbl_edge'])]
print('files changed in key metrics:', len(diff), diff[:12])
PY
