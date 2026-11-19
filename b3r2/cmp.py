import sys,re
def load(p):
    d={}
    seen=set()
    for l in open(p):
        m=re.match(r'^(\S+) (static|routed) ([\d.]+)x([\d.]+) (.*)$',l)
        if not m: continue
        k=(m.group(1),m.group(2))
        if k in d: k=(m.group(1)+'#2',m.group(2))
        kv=dict(x.split('=') for x in m.group(5).split() if '=' in x)
        kv['w']=m.group(3); kv['h']=m.group(4)
        d[k]=kv
    return d
a=load(sys.argv[1]); b=load(sys.argv[2])
cols=sys.argv[3].split(',') if len(sys.argv)>3 else ['x_drawn','lbl_edge','lbl_node','end_tilt','bad_angle','own_frame_x','edge_node_x','w','h']
tot={}
for k in b:
    if k not in a: continue
    diffs=[]
    for c in cols:
        if a[k].get(c)!=b[k].get(c):
            try:
                if abs(float(a[k][c])-float(b[k][c]))<0.05: continue
            except: pass
            diffs.append(f"{c} {a[k].get(c)}->{b[k].get(c)}")
    if diffs: print(k[0],k[1],' '.join(diffs))
