#!/usr/bin/env python3
import csv, io, os, sys, glob, collections
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r2/bulk/all.tsv'
HZ=['edge_node_x','shared_ports','arrow_on_start','wiggles','frame_side','node_overlap','node_frame','frame_overlap','own_frame_x']
prev={}
for line in open(V):
    f=line.rstrip('\n').split('\t')
    name, mode = f[0], f[1]
    d={'bm': f[2]=='bm=1'}
    i=4
    while i+2 < len(f)+1 and i < len(f):
        k=f[i]
        if k=='hard': d['hard']=(int(f[i+1]),int(f[i+2])); d['kinds']=f[i+3]; i+=4; continue
        d[k]=(float(f[i+1]),float(f[i+2])); i+=3
    prev[(name,mode)]=d
now={}
for p in glob.glob('now/*.tsv'):
    for r in csv.DictReader(open(p),delimiter='\t'):
        if r['file']=='TOTAL': continue
        now[(r['file'],r['mode'])]=r
tot=collections.defaultdict(lambda:[0,0,0]); added=[]; worse_fan=better_fan=0; kinds=collections.Counter()
for key,d in prev.items():
    if key not in now or not d['bm']: continue
    r=now[key]; mode=key[1]
    h=sum(int(float(r[c])) for c in HZ)
    t=tot[(mode,'hard')]; t[0]+=d['hard'][0]; t[1]+=d['hard'][1]; t[2]+=h
    for k,c in [('fan_x','fan_x'),('x_drawn','x_drawn'),('efp','edge_frame_pairs'),('ovl','overlap_pairs'),('defects','defects')]:
        if k in d:
            t=tot[(mode,k)]; t[0]+=d[k][0]; t[1]+=d[k][1]; t[2]+=float(r[c])
    if mode=='static':
        if h> d['hard'][0]:
            ks=[c for c in HZ if float(r[c])>0]
            added.append((key[0],d['hard'][0],h,ks))
            for c in HZ:
                if float(r[c])>0: kinds[c]+=1
        f0=d['fan_x'][0]; fn=float(r['fan_x'])
        worse_fan+= fn>f0; better_fan+= fn<f0
print('block-mode inputs compared:', sum(1 for k,d in prev.items() if d['bm'] and k in now and k[1]=='static'))
for (mode,k),v in sorted(tot.items()):
    print(f'{mode:7} {k:8} off {v[0]:g}  prev-round {v[1]:g}  now {v[2]:g}')
print('static fan_x worse than off on', worse_fan, 'better on', better_fan)
print('static hard zeros added vs off on', len(added), 'inputs; kinds:', dict(kinds))
non_note=[a for a in added if any(k not in ('shared_ports','arrow_on_start') for k in a[3])]
print('of which with a kind other than shared ports / arrows on starts:', len(non_note))
for a in non_note: print('  ', a)
