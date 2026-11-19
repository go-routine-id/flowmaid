import csv,sys
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/'
V=S+'verify/c-leftovers/r2/perf/'
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
hb,base=load(S+'runs/baseline/perf.tsv')
runs={n:load(V+n+'.tsv') for n in sys.argv[1:]}
names=sys.argv[1:]
h0=runs[names[0]][0]
timing={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','partial_ms','partial_mean'}
print('timing cols present:',[c for c in h0 if 'ms' in c or 'drag' in c or 'partial' in c])
# quality identity pre vs now
pre=[n for n in names if n.startswith('pre')]; now=[n for n in names if n.startswith('now')]
tcols=[c for c in h0 if 'ms' in c or 'drag' in c or 'partial' in c or c in('route_ms',)]
diffq=[]
for k in runs[now[0]][1]:
    a=runs[pre[0]][1].get(k); b=runs[now[0]][1][k]
    if a is None: diffq.append((k,'missing in pre')); continue
    for c in h0:
        if c in tcols: continue
        if a.get(c)!=b.get(c): diffq.append((k,c,a.get(c),b.get(c)))
print('quality metric differences pre vs now:',diffq[:20], len(diffq))
for col in ['route_ms','drag_ms','scene_ms']+[c for c in tcols if 'partial' in c]:
    print('==',col)
    for k in sorted(runs[now[0]][1]):
        if col=='scene_ms' and k[1]!='static': continue
        if col!='scene_ms' and k[1]!='routed': continue
        b=base.get(k,{}).get(col)
        vals=[runs[n][1].get(k,{}).get(col,'?') for n in names]
        lim=''
        flag=''
        try:
            bf=float(b)
            limit = 2*bf if col=='scene_ms' else max(1.25*bf,bf+1.0)
            lim='%.2f'%limit
            nv=[float(runs[n][1][k][col]) for n in now]; pv=[float(runs[n][1][k][col]) for n in pre]
            if min(nv)>limit: flag+=' OVER(now both)'
            elif max(nv)>limit: flag+=' over(now once)'
            if min(pv)>limit: flag+=' [pre over too]'
        except Exception as e: pass
        print('  %-20s base %-6s limit %-6s  '%(k[0],b,lim)+'  '.join(f'{n}={v}' for n,v in zip(names,vals))+flag)
