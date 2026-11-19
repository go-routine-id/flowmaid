import csv,sys,os
# usage: cmp.py dir tag1 tag2 ... ; prints per-file defects/efx/chip static|routed for q034,q035,qnow,tags
d=sys.argv[1]; tags=['034','035']+sys.argv[2:]
def load(p):
    if not os.path.exists(p): return {}
    rows=list(csv.reader(open(p),delimiter='\t'))
    if not rows: return {}
    h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h) and r[0]!='TOTAL'}
ds=[load(f'{d}/q{t}.tsv' if t in('034','035','now') else f'{d}/q_{t}.tsv') for t in tags]
keys=sorted(set(k for k in ds[0]))
cols=os.environ.get('COLS','defects,edge_frame_x,edge_chip,width').split(',')
print('tags:', ' / '.join(tags))
tot={}
for k in keys:
    if os.environ.get('MODE') and k[1]!=os.environ['MODE']: continue
    line=f"{k[0][:26]:26s} {k[1][0]} "
    for c in cols:
        vals=[dd.get(k,{}).get(c,'-') for dd in ds]
        if c=='width': vals=[v.split('.')[0] for v in vals]
        line+=f" {c[:6]}={'/'.join(vals)}"
        for i,v in enumerate(vals):
            try: tot[(k[1],c,i)]=tot.get((k[1],c,i),0)+float(v)
            except: pass
    print(line)
for mode in ('static','routed'):
    print(mode, ' '.join(f"{c}: "+'/'.join(f"{tot.get((mode,c,i),0):.0f}" for i in range(len(tags))) for c in cols if c!='width'))
