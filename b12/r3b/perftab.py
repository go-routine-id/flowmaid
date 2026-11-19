import csv,sys,re,os
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
def load(lab):
    p=f'{S}/runs/{lab}/perf.tsv'
    if not os.path.exists(p): return None
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
lim={'advanced':(1.42,1.72,2.61),'subgraph':(0.38,1.05,2.04),'fanout-frames':(4.38,1.34,3.44),'dense-lr':(46.8,8.03,17.76),'dense-td':(50.68,9.20,17.80),'bench-75':(61.94,15.76,25.28),'infra-overview':(13.2,2.9,7.25)}
# public fixture: rule on the start's default runs (lower of two)
st=[load('b12a3-route-off-start-1'),load('b12a3-route-off-start-2')]
for f in ['blocks-overview','infra-overview-v2']:
    s=min(float(x[(f,'static')]['scene_ms']) for x in st); r=min(float(x[(f,'routed')]['route_ms']) for x in st); d=min(float(x[(f,'routed')]['drag_ms']) for x in st)
    lim[f]=(max(2*s,s+0.3) if s<1 else 2*s, max(1.25*r,r+1), max(1.25*d,d+1))
labs=sys.argv[1:]
data={l:load(l) for l in labs}
print('| file | limit scene / route / drag | '+' | '.join(labs)+' |')
print('|---|---|'+'---|'*len(labs))
for f,(ls,lr,ld) in lim.items():
    cells=[]
    for l in labs:
        d=data[l]
        if not d or (f,'routed') not in d: cells.append('-'); continue
        s=float(d[(f,'static')]['scene_ms']); r=float(d[(f,'routed')]['route_ms']); dr=float(d[(f,'routed')]['drag_ms'])
        mark=lambda v,L: f'**{v:.2f}**' if v>L else f'{v:.2f}'
        cells.append(f'{mark(s,ls)} / {mark(r,lr)} / {mark(dr,ld)}')
    print(f'| {f} | {ls:.2f} / {lr:.2f} / {ld:.2f} | '+' | '.join(cells)+' |')
for l in labs:
    p=f'{S}/runs/{l}/rp.txt'
    if os.path.exists(p):
        lines=[re.sub(r' \(route\(\) reported.*','',x.strip()) for x in open(p) if 'route_partial worst' in x]
        box=open(f'{S}/runs/{l}/box.txt').read().replace('\n',' | ')
        print(f'- {l}: '+'; '.join(lines)+f'  [{box}]')
