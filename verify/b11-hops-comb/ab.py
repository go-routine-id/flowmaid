import sys,csv,glob
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,[dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)]
V=sys.argv[1]
hb,base=load(sys.argv[2])
bm={(r[hb[0]],r['mode']):r for r in base}
def mins(prefix):
    out={}
    for p in sorted(glob.glob(f'{V}/perf/{prefix}*.tsv')):
        h,rows=load(p)
        for r in rows:
            k=(r[h[0]],r['mode'])
            for c in ['route_ms','scene_ms','drag_ms','drag_mean']:
                v=float(r[c])
                out.setdefault(k,{}).setdefault(c,[]).append(v)
    return out
pre=mins('pre'); now=mins('now')
print('file mode col base pre(min/max) now(min/max) limit verdict')
for k in now:
    for c in ['route_ms','scene_ms','drag_ms']:
        b=float(bm[k][c]) if k in bm else 0
        if b==0 and max(now[k][c])==0: continue
        lim = 2*b if c=='scene_ms' else max(1.25*b,b+1.0)
        n=now[k][c]; p=pre[k][c]
        v='OK' if min(n)<=lim else 'OVER'
        if max(n)>lim and v=='OK': v='ok(min) over(max)'
        print(f"{k[0]:14s} {k[1]:7s} {c:9s} base={b:6.2f} pre={min(p):6.2f}/{max(p):6.2f} now={min(n):6.2f}/{max(n):6.2f} lim={lim:6.2f} {v}  now/pre={min(n)/max(min(p),1e-9):.2f}")
