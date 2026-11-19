#!/usr/bin/env python3
"""tcmp.py A B : static scene_ms per file (block-mode rows), ratio distribution"""
import sys, glob, os
def load(p):
    L=[l.rstrip('\n').split('\t') for l in open(p)]
    h=L[0]; i=h.index('scene_ms'); bm=h.index('block_mode') if 'block_mode' in h else None
    return {r[0]:(float(r[i]), r[bm] if bm is not None else '?') for r in L[1:] if len(r)==len(h) and r[1]=='static' and r[0]!='TOTAL'}
A,B=sys.argv[1],sys.argv[2]
n=0; slow=[]; ta=tb=0
for pa in sorted(glob.glob(A+'/q-*.tsv')):
    pb=os.path.join(B,os.path.basename(pa))
    if not os.path.exists(pb): continue
    a,b=load(pa),load(pb)
    for k in a:
        if k not in b: continue
        n+=1; ta+=a[k][0]; tb+=b[k][0]
        if b[k][0] > 2*a[k][0] and b[k][0]-a[k][0] > 0.3: slow.append((b[k][0]/max(a[k][0],1e-3), os.path.basename(pa), k, a[k][0], b[k][0]))
print('files', n, 'total ms %.1f -> %.1f'%(ta,tb), '>2x slower (and +0.3ms):', len(slow))
for s in sorted(slow, reverse=True)[:15]: print('  %.2fx %s %s %.2f -> %.2f'%s)
