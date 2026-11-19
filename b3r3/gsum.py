import csv,sys
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'));h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
fs=sys.argv[1:]
ds=[load(f) for f in fs]
cols=['edge_node_x','edge_edge_x','end_tilt','end_gaps','shared_ports','arrow_on_start','own_frame_x','defects','lbl_edge','lbl_node','overlap_pairs','lbl_arrow']
for mode in ['static','routed']:
  for col in cols:
    vals=[sum(float(v[col]) for k,v in d.items() if k[1]==mode and not k[0].upper().startswith('TOTAL')) for d in ds]
    print(mode,col,*vals)
  n180=[sum(1 for k,v in d.items() if k[1]==mode and not k[0].upper().startswith('TOTAL') and float(v['end_tilt_max'])>=100) for d in ds]
  print(mode,'tilt>=100',*n180)
