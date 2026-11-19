import sys,csv
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
a=load(sys.argv[1]); b=load(sys.argv[2]); cols=sys.argv[3].split(',')
lim=int(sys.argv[4]) if len(sys.argv)>4 else 30
rows=[]
for k in a:
    if k not in b or k[0]=='TOTAL': continue
    d=[(c,float(a[k][c]),float(b[k][c])) for c in cols]
    if any(x!=y for _,x,y in d) or a[k]['width']!=b[k]['width'] or a[k]['height']!=b[k]['height']:
        rows.append((sum(y-x for _,x,y in d),k,d,a[k]['width'],b[k]['width'],a[k]['height'],b[k]['height']))
rows.sort(key=lambda r:-r[0])
for r in rows[:lim]:
    print(f"{r[1][0]:40s} {r[1][1]:6s} " + ' '.join(f"{c}:{x:g}->{y:g}" for c,x,y in r[2]) + f"  w {float(r[3]):.0f}->{float(r[4]):.0f} h {float(r[5]):.0f}->{float(r[6]):.0f}")
