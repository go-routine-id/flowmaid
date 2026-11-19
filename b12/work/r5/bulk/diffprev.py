import csv, glob, collections
exec(open('cmp.py').read().split("tot=collections")[0])
rows=[]
for key,d in prev.items():
    if key not in now or not d['bm'] or key[1]!='static': continue
    r=now[key]
    for k,c in [('fan_x','fan_x'),('x_drawn','x_drawn')]:
        a,b,n=d[k][0],d[k][1],float(r[c])
        if n!=b: rows.append((k,key[0],a,b,n))
for k in ['fan_x','x_drawn']:
    sel=[r for r in rows if r[0]==k]
    up=[r for r in sel if r[4]>r[3]]; dn=[r for r in sel if r[4]<r[3]]
    print(k,'changed vs prev round on',len(sel),'up',len(up),'(+%g)'%sum(r[4]-r[3] for r in up),'down',len(dn),'(%g)'%sum(r[4]-r[3] for r in dn))
    for r in sorted(up,key=lambda r:r[3]-r[4])[:8]: print('   up',r)
