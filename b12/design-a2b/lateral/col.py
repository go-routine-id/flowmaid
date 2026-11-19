# col.py <col> <variant...>: one column per variant, rows = inputs that differ in any variant
import sys,csv
col=sys.argv[1]; vs=sys.argv[2:]; L='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/lateral/runs'
tabs={}
for v in vs:
    rows=list(csv.DictReader(open(f'{L}/{v}/metrics.tsv'),delimiter='\t'))
    tabs[v]={r['name']:r.get(col,'') for r in rows}
names=sorted(tabs[vs[0]])
print('%-36s'%'name'+''.join('%10s'%v for v in vs))
for n in names:
    vals=[tabs[v].get(n,'') for v in vs]
    if len(set(vals))>1 or (sys.argv[0] and any(x not in ('0','') for x in vals) and col=='lateral_x'):
        print('%-36s'%n+''.join('%10s'%x for x in vals))
