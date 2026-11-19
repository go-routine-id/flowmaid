import sys,csv
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
b=load(S+'/runs/baseline/perf.tsv')
cur=[load(S+'/runs/'+l+'/perf.tsv') for l in sys.argv[1:]]
cols=sys.argv[1:]
bad=[]
print('file mode | route_ms drag_ms scene_ms (base / limit / '+' '.join(cols)+')')
for k in sorted(b):
    if k[0]=='TOTAL': continue
    o=b[k]
    out=[]
    for c,(lim) in [('route_ms',lambda x:max(1.25*x,x+1.0)),('drag_ms',lambda x:max(1.25*x,x+1.0)),('scene_ms',lambda x:2*x)]:
        bv=float(o[c]);
        if bv==0: continue
        L=lim(bv)
        vals=[float(r[k][c]) if k in r else float('nan') for r in cur]
        flag='!' if all(v>L for v in vals) else ''
        out.append(f"{c} {bv:.2f}/{L:.2f}/{' '.join(f'{v:.2f}' for v in vals)}{flag}")
    print(k[0],k[1],' | '.join(out))
