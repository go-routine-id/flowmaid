import subprocess,re,os
Q='../tree/tests/fixtures/quality'
env=dict(os.environ,FM_B12='2',FM_B12_LVL='1',FM_B12_STARTS='7',FM_B12_TRIG='1')
def width(src,tag,exp=True):
    p=f'shuf/{tag}.mmd'; open(p,'w').write(src)
    o=f'shuf/{tag}.svg'
    if exp: subprocess.run(['./fm-spec',p,'-o',o],env=env,capture_output=True)
    else: subprocess.run(['../fm038',p,'-o',o],capture_output=True)
    s=open(o).read()
    m=re.search(r'viewBox="[\d.\-]+ [\d.\-]+ ([\d.]+) ([\d.]+)"',s)
    return float(m.group(1))
src=open(f'{Q}/dense-td.mmd').read()
lines=src.split('\n')
if lines[-1]=='': lines=lines[:-1]
at=[i for i,l in enumerate(lines) if l.lstrip().startswith('n45[')][0]
rest=list(lines); decl=rest.pop(at)
depth=0; tops=[]
for i,l in enumerate(rest):
    t=l.strip()
    if i>=2 and depth==0: tops.append(i)
    if t.startswith('subgraph'): depth+=1
    elif t=='end': depth-=1
tops.append(len(rest))
for exp in (False,True):
    w0=width('\n'.join(lines),'n45-0',exp)
    out=[]
    for k in range(6):
        i=tops[k*(len(tops)-1)//5]
        moved=list(rest); moved.insert(i,decl)
        w=width('\n'.join(moved),f'n45-{k}',exp)
        out.append(f'{w:.0f} ({(w-w0)/w0*100:+.1f}%)')
    print('exp' if exp else '038',w0,out)
