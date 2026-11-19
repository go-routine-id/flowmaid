# usage: perm.py <src.mmd> <outdir> ; writes e<seed>.mmd edge-line shuffles (12) 
import sys, random, os
src=open(sys.argv[1]).read().split('\n'); out=sys.argv[2]; os.makedirs(out,exist_ok=True)
name=os.path.basename(sys.argv[1])[:-4]
is_edge=lambda l: any(a in l for a in ['-->','-.->','==>','~~~','---'])
at=[i for i,l in enumerate(src) if is_edge(l) and not l.strip().startswith('%%')]
for seed in range(1,13):
    r=random.Random(seed); order=[src[i] for i in at]; r.shuffle(order)
    o=list(src)
    for i,l in zip(at,order): o[i]=l
    open(f"{out}/{name}.e{seed:02d}.mmd","w").write('\n'.join(o))
