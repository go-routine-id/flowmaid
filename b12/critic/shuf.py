import sys,subprocess,re,os
M=(1<<64)-1
Q='../tree/tests/fixtures/quality'
env=dict(os.environ,FM_B12='2',FM_B12_LVL='1',FM_B12_STARTS='7',FM_B12_TRIG='1')
def width(src,tag,exp=True):
    p=f'shuf/{tag}.mmd'; open(p,'w').write(src)
    o=f'shuf/{tag}.{"exp" if exp else "038"}.svg'
    if exp: subprocess.run(['./fm-spec',p,'-o',o],env=env,capture_output=True)
    else: subprocess.run(['../fm038',p,'-o',o],capture_output=True)
    s=open(o).read()
    m=re.search(r'viewBox="[\d.\-]+ [\d.\-]+ ([\d.]+) ([\d.]+)"',s)
    return float(m.group(1)),float(m.group(2))
for name in ['dense-td','dense-lr','bench-75']:
    src=open(f'{Q}/{name}.mmd').read()
    lines=src.split('\n')
    if lines and lines[-1]=='': lines=lines[:-1]
    at=[i for i,l in enumerate(lines) if any(a in l for a in ('-->','-.->','==>'))]
    for exp in (False,True):
        w0=width('\n'.join(lines),name+'-s0',exp)
        res=[]
        for seed in range(1,7):
            x=(seed*0x9e3779b97f4a7c15)&M
            order=[lines[i] for i in at]
            for i in range(len(order)-1,0,-1):
                x^=(x<<13)&M; x^=x>>7; x^=(x<<17)&M
                j=x%(i+1); order[i],order[j]=order[j],order[i]
            out=list(lines)
            for i,l in zip(at,order): out[i]=l
            w=width('\n'.join(out),f'{name}-s{seed}',exp)
            res.append(w)
        print(name,'exp' if exp else '038','w0',w0,'shuffles',[f'{w[0]:.0f}x{w[1]:.0f} ({(w[0]-w0[0])/w0[0]*100:+.1f}%)' for w in res])
