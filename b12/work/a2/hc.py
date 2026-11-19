#!/usr/bin/env python3
# hc.py a.tsv b.tsv [cols] [mode]: rows (static by default) whose cols differ
import sys,csv
def load(p):
    r=list(csv.reader(open(p),delimiter='\t')); h=r[0]
    return h,{(x[0],x[1]):dict(zip(h,x)) for x in r[1:] if x and x[0]!='TOTAL' and len(x)>2}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
cols=(sys.argv[3] if len(sys.argv)>3 and sys.argv[3] else 'x_drawn,fan_x,defects,overlap_pairs,min_edge_gap,lateral_x,side_ends,frame_overlap,node_frame,title_cut,compact_max,sib_stagger,width,height').split(',')
mode=sys.argv[4] if len(sys.argv)>4 else 'static'
for k in sorted(b):
    if k[1]!=mode: continue
    d=[c for c in cols if a.get(k,{}).get(c)!=b[k].get(c)]
    if d: print(f"{k[0]:40s}", '  '.join(f"{c}={a.get(k,{}).get(c,'?')}->{b[k].get(c)}" for c in d))
