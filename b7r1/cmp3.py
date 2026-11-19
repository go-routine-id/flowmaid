import sys
def load(p):
    d={}
    for l in open(p):
        if l.startswith('TOT'):
            _,f,a,b=l.rstrip('\n').split('\t'); d[f]=(float(a),float(b))
    return d
pre,head,now=[load(f'/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b7r1/{x}.txt') for x in sys.argv[1:4]]
keys=sys.argv[4].split(',') if len(sys.argv)>4 else sorted(now)
print(f"{'metric':16s} {'pre_s':>9s} {'head_s':>9s} {'now_s':>9s} | {'pre_r':>9s} {'head_r':>9s} {'now_r':>9s}")
for k in keys:
    if k not in now: continue
    p,h,n=pre.get(k,(0,0)),head.get(k,(0,0)),now[k]
    print(f"{k:16s} {p[0]:9.1f} {h[0]:9.1f} {n[0]:9.1f} | {p[1]:9.1f} {h[1]:9.1f} {n[1]:9.1f}")
