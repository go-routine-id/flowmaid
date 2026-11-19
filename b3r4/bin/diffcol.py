import csv,sys
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):r for r in rows[1:]}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
for c in sys.argv[3].split(','):
    i=h.index(c)
    for k in sorted(a):
        if k[0]=='TOTAL' or k not in b: continue
        if a[k][i]!=b[k][i]: print(c,k[0],k[1],a[k][i],'->',b[k][i])
