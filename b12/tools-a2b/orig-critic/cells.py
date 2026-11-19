import sys,re
def load(p):
    d={}
    for l in open(p):
        if l.startswith('MUST FAIL'):
            f=l.split()
            name,met,val=f[2],f[3],f[4]
            m=re.search(r'must ([<>]=) (\S+)',l)
            d[(name,met)]=(float(val),m.group(1),float(m.group(2)))
    return d
a=load(sys.argv[1]); b=load(sys.argv[2])
worse=[];better=[];new=[];gone=[]
for k in sorted(set(a)|set(b)):
    if k in a and k in b:
        va,op,_=a[k]; vb=b[k][0]
        if va!=vb:
            w = (vb>va) if op=='<=' else (vb<va)
            (worse if w else better).append('%s %s %g->%g'%(k[0],k[1],va,vb))
    elif k in b: new.append('%s %s %g (bound %s %g)'%(k[0],k[1],b[k][0],b[k][1],b[k][2]))
    else: gone.append('%s %s %g'%(k[0],k[1],a[k][0]))
print('NEW (%d): '%len(new)+'; '.join(new))
print('GONE (%d): '%len(gone)+'; '.join(gone))
print('WORSE inside failing (%d): '%len(worse)+'; '.join(worse))
print('BETTER inside failing (%d): '%len(better)+'; '.join(better))
