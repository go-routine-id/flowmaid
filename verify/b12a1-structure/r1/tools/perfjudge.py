import sys,csv
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
hb,base=load(S+'/runs/baseline/perf.tsv')
hs,st=load(S+'/runs/b12-start/perf.tsv')
hs2,st2=load(S+'/runs/b12-start-2/perf.tsv')
runs=[load(S+'/runs/'+l+'/perf.tsv') for l in sys.argv[1:]]
T=['route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic']
# non-timing equality vs start
for (h,c) in runs:
    diffs=0
    for k,r in c.items():
        s=st.get(k)
        if not s: print('no start row',k); continue
        for col in h:
            if col in T or col in('file','mode'): continue
            if r[col]!=s.get(col):
                diffs+=1; print('DIFF',k,col,s.get(col),'->',r[col])
    print('non-timing diffs vs b12-start:',diffs, 'rows',len(c))
print('file mode | col base start start2 | runs... | limit | verdict')
for k in sorted(runs[0][1]):
    if k[0]=='TOTAL': continue
    for col in ['scene_ms','route_ms','drag_ms']:
        b=base.get(k,{}).get(col); 
        vals=[float(c[k][col]) for (h,c) in runs]
        s1=float(st[k][col]) if k in st else None; s2=float(st2[k][col]) if k in st2 else None
        if all(v==0 for v in vals): continue
        if b is None:
            ref=min(x for x in (s1,s2) if x is not None); src='start'
        else:
            ref=float(b); src='base'
        if col=='scene_ms':
            lim=2*ref if ref>=1 else max(2*ref,ref+0.3)
            lim_plan=2*ref
        else:
            lim=max(1.25*ref,ref+1.0); lim_plan=lim
        over=[v>lim for v in vals]
        flag='OVER-BOTH' if all(over) else ('over-one' if any(over) else 'ok')
        if col=='scene_ms' and any(v>lim_plan for v in vals) and flag=='ok': flag='ok(D1; over PLAN 2x)'
        print(f"{k[0]:20s} {k[1]:7s} {col:9s} {src} {ref:7.2f} start {s1} {s2} | now {' '.join('%.2f'%v for v in vals)} | lim {lim:.2f} | {flag}")
