import re,sys,math
exec(open('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10/xcount.py').read().split("svg=open")[0])
def edges(path):
    svg=open(path).read()
    # flowmaid: <g> per edge with <title>a → b</title>? find path + title
    out=[]
    for m in re.finditer(r'<path d="(M [^"]+)"[^>]*stroke-width[^>]*>(?:<title>([^<]*)</title>)?',svg):
        out.append((m.group(2) or '?', sample(m.group(1))))
    return out
def per(es):
    cnt={}
    tot=0
    for i in range(len(es)):
        for j in range(i+1,len(es)):
            A,B=es[i][1],es[j][1];hits=[]
            for k in range(len(A)-1):
                for l in range(len(B)-1):
                    p=inter(A[k],A[k+1],B[l],B[l+1])
                    if p and all(math.dist(p,q)>4 for q in hits) and min(math.dist(p,A[0]),math.dist(p,A[-1]),math.dist(p,B[0]),math.dist(p,B[-1]))>6: hits.append(p)
            for n in (es[i][0],es[j][0]): cnt[n]=cnt.get(n,0)+len(hits)
            tot+=len(hits)
    return tot,cnt
a=edges(sys.argv[1]);b=edges(sys.argv[2])
ta,ca=per(a);tb,cb=per(b)
print('total',ta,'->',tb)
names=sorted(set(ca)|set(cb),key=lambda n:cb.get(n,0)-ca.get(n,0),reverse=True)
for n in names[:15]: print(n,ca.get(n,0),'->',cb.get(n,0))
