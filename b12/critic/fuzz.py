import random,subprocess,re,os,sys
env=dict(os.environ,FM_B12='2',FM_B12_LVL='1',FM_B12_STARTS='7',FM_B12_TRIG='1')
def gen(seed):
    r=random.Random(seed)
    nb=r.randint(3,4)
    L=['flowchart TD','  H[hub]']
    blocks=[]
    for b in range(nb):
        k=r.randint(2,4)
        ids=[f'{"abcd"[b]}{i}' for i in range(k)]
        blocks.append(ids)
        L.append(f'  subgraph B{b}[Block {b}]')
        for i in range(k-1): L.append(f'    {ids[i]} --> {ids[i+1]}')
        L.append('  end')
    for ids in blocks: L.append(f'  H --> {ids[0]}')
    nx=r.randint(2,4)
    for x in range(nx):
        L.append(f'  X{x}[src {x}]')
        for _ in range(r.randint(2,3)):
            ids=r.choice(blocks); L.append(f'  X{x} --> {r.choice(ids)}')
    nz=r.randint(1,3)
    for z in range(nz):
        L.append(f'  Z{z}[sink {z}]')
        for _ in range(r.randint(2,3)):
            ids=r.choice(blocks); L.append(f'  {r.choice(ids)} --> Z{z}')
    return '\n'.join(L)+'\n'
def rects(svg):
    out=[]
    for m in re.finditer(r'<rect x="([\d.\-]+)" y="([\d.\-]+)" width="([\d.]+)" height="([\d.]+)" rx="8" fill="#[0-9a-f]+" stroke="#[0-9a-f]+" stroke-width="1.4"',svg):
        x,y,w,h=map(float,m.groups()); out.append((x,y,x+w,y+h))
    return out
bad=0; on=0
for seed in range(int(sys.argv[1]),int(sys.argv[2])):
    src=gen(seed); p=f'fuzz/f{seed}.mmd'; open(p,'w').write(src)
    o=f'fuzz/f{seed}.svg'
    pr=subprocess.run(['./fm-spec',p,'-o',o],env=env,capture_output=True,text=True,timeout=60)
    if pr.returncode!=0:
        print('FAIL',seed,pr.stderr[-200:]); bad+=1; continue
    svg=open(o).read(); R=rects(svg)
    ov=[(i,j) for i in range(len(R)) for j in range(i+1,len(R)) if R[i][0]<R[j][2]-0.5 and R[j][0]<R[i][2]-0.5 and R[i][1]<R[j][3]-0.5 and R[j][1]<R[i][3]-0.5]
    if ov:
        print('OVERLAP',seed,len(R),ov); bad+=1
print('done bad',bad)
