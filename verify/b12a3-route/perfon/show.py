import csv,sys,glob,os
d=os.path.dirname(os.path.abspath(__file__))
runs=['head-1','head-2','base-1','base-2']
data={}
for r in runs:
    p=os.path.join(d,r+'.tsv')
    if not os.path.exists(p): continue
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]
    for row in rows[1:]:
        if len(row)<len(h): continue
        m=dict(zip(h,row))
        key=(m[h[0]],m[h[1]])
        data.setdefault(key,{})[r]=m
limits={'advanced':(1.42,1.72,2.61),'subgraph':(0.38,1.05,2.04),'fanout-frames':(4.38,1.34,3.44),'dense-lr':(46.8,8.03,17.76),'dense-td':(50.68,9.20,17.80),'bench-75':(61.94,15.76,25.28),'infra-overview':(13.2,2.9,7.25)}
print("file mode | scene_ms h1 h2 b1 b2 | route_ms h1 h2 b1 b2 | drag_ms h1 h2 b1 b2 | dfr_own h/b | dfr_far h/b | limits")
files=sorted(set(k[0] for k in data))
for f in files:
    st=data.get((f,'static'),{}); ro=data.get((f,'routed'),{})
    g=lambda D,r,c: D.get(r,{}).get(c,'-')
    print(f, '|', *[g(st,r,'scene_ms') for r in runs], '|', *[g(ro,r,'route_ms') for r in runs], '|', *[g(ro,r,'drag_ms') for r in runs], '|', g(ro,'head-1','dfr_own'), g(ro,'base-1','dfr_own'), '|', g(ro,'head-1','dfr_far'), g(ro,'base-1','dfr_far'), '|', limits.get(f,''))
