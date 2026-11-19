import csv,sys
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
a=load(sys.argv[1]); b=load(sys.argv[2]); col=sys.argv[3]; mode=sys.argv[4] if len(sys.argv)>4 else 'static'
d=[]
for k in b:
    if k[1]!=mode or k[0]=='TOTAL' or k not in a: continue
    x=float(b[k][col])-float(a[k][col])
    if x!=0: d.append((x,k[0],a[k][col],b[k][col]))
d.sort()
for r in d[-int(sys.argv[5]) if len(sys.argv)>5 else -15:]: print(*r)
print('sum+',sum(x[0] for x in d if x[0]>0),'sum-',sum(x[0] for x in d if x[0]<0))
