#!/usr/bin/env python3
"""cmp2.py A.tsv B.tsv [mode] : per-file verdict columns + timings, A/B"""
import sys
def load(p):
    L=[l.rstrip('\n').split('\t') for l in open(p)]
    h=L[0]; return h,{(r[0],r[1]):r for r in L[1:] if len(r)==len(h)}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2]); mode=sys.argv[3] if len(sys.argv)>3 else 'static'
cols=['defects','x_drawn','lateral_x','fan_x','edge_chip','side_ends','min_edge_gap','scene_ms','route_ms']
cols=[c for c in cols if c in h]
idx=[h.index(c) for c in cols]
print('%-30s'%'file', ' '.join('%-13s'%c[:13] for c in cols))
for k in sorted(a):
    if k[1]!=mode or k not in b: continue
    ra,rb=a[k],b[k]
    print('%-30s'%k[0][-30:], ' '.join('%-13s'%('%s/%s'%(ra[i],rb[i]) if ra[i]!=rb[i] else ra[i]) for i in idx))
