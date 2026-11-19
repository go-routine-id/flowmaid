import csv,sys
def load(f):
    rows={}; hdr=None
    for r in csv.reader(open(f),delimiter='\t'):
        if r[0]=='dir': hdr=r; continue
        rows[(r[0],r[1],r[2])]=dict(zip(hdr,r))
    return rows
a=load(sys.argv[1]); b=load(sys.argv[2])
HARD=['edge_node_x','shared_ports','arrow_on_start','wiggles','frame_side','node_overlap','node_frame','frame_overlap']
cols=['x_drawn','defects','edge_frame_pairs','overlap_pairs','fan_x']
tot={c:[0,0] for c in cols+['hard']}
for k in sorted(b):
    if k not in a: continue
    ra,rb=a[k],b[k]
    ha=sum(int(float(ra[c])) for c in HARD); hb=sum(int(float(rb[c])) for c in HARD)
    d=[]
    for c in cols:
        va,vb=int(float(ra[c])),int(float(rb[c])); tot[c][0]+=va; tot[c][1]+=vb
        if va!=vb: d.append(f'{c} {va}->{vb}')
    tot['hard'][0]+=ha; tot['hard'][1]+=hb
    if ha!=hb: d.append(f'hard {ha}->{hb}')
    if d and (len(sys.argv)<4 or sys.argv[3] in k[2]): print('%-10s %-34s %-6s'%k, '; '.join(d))
print('TOTAL', {c:tuple(v) for c,v in tot.items()})
