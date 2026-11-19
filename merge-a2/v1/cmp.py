import sys, glob
def load(fs):
    d={}
    for f in fs:
        for l in open(f, encoding='utf-8', errors='replace'):
            l=l.rstrip('\n')
            if not l: continue
            p=l.split('\t',1)[0]; d[p]='\t'.join(x for x in l.split('\t') if not x.startswith('doc='))
    return d
a=load(sorted(glob.glob(sys.argv[1]))); b=load(sorted(glob.glob(sys.argv[2])))
common=sorted(set(a)&set(b))
diff=[p for p in common if a[p]!=b[p] and 'EXIT=' not in a[p] and 'EXIT=' not in b[p]]
ex=[p for p in common if 'EXIT=' in a[p] or 'EXIT=' in b[p]]
fc=sum(1 for p in common if 'scene.svg=' in a[p])
cls=sum(1 for p in common if 'class.svg=' in a[p]); er=sum(1 for p in common if 'er.svg=' in a[p])
err=sum(1 for p in common if 'render=ERR' in a[p])
print(f"common {len(common)}; flow/state with geometry {fc}; class {cls}; er {er}; render errors {err}; differences {len(diff)}; exits {len(ex)}")
for p in diff[:20]:
    fa=a[p].split('\t'); fb=b[p].split('\t')
    print("DIFF", p, [x for x,y in zip(fa,fb) if x!=y][:6], len(fa), len(fb))
for p in ex: print("EXIT", p, a[p].split('\t')[-1][:40] if 'EXIT=' in a[p] else 'ok', '|', b[p].split('\t')[-1][:40] if 'EXIT=' in b[p] else 'ok')
