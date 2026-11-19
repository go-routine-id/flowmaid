import re, subprocess, os, json, sys
ROOT='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
WT=ROOT+'/wt/rel39'
B=ROOT+'/compare/fm039-before-pickfix'
OUT=ROOT+'/final-review/out'
os.makedirs(OUT,exist_ok=True)
new=open(WT+'/README.md',encoding='utf-8').read()
def run(args,inp=None,cwd=WT):
    p=subprocess.run([B]+args,input=inp,capture_output=True,text=True,cwd=cwd)
    return p.returncode,p.stdout,p.stderr
res=[]
def ok_svg(name,src,args=None,expect_err=None):
    if args is None: args=[]
    rc,out,err=run(args,inp=src)
    good = rc==0 and out.lstrip().startswith('<svg') and out.rstrip().endswith('</svg>')
    if expect_err is not None:
        good = rc!=0 and expect_err in err
    res.append((name,good,rc,err.strip()[:160]))
    open(OUT+'/'+re.sub(r'[^\w.-]','_',name)+'.svg','w').write(out)
    return rc,out,err
# fenced blocks
blocks=re.findall(r'```(\w*)\n(.*?)```',new,flags=re.S)
print('fenced blocks',[(l,len(b)) for l,b in blocks])
n=0
for lang,body in blocks:
    n+=1
    if lang=='mmd':
        p=OUT+f'/block{n}.mmd'; open(p,'w').write(body)
        rc,out,err=run(['--advance-text',p]); res.append((f'block{n} advance-text',rc==0 and out.startswith('<svg'),rc,err[:160]))
    elif lang=='json':
        p=OUT+f'/block{n}.json'; open(p,'w').write(body)
        rc,out,err=run(['--advance',p]); res.append((f'block{n} advance json',rc==0 and out.startswith('<svg'),rc,err[:160]))
    elif lang=='rust':
        for m in re.finditer(r'render_svg(?:_advanced)?\("((?:[^"\\]|\\.)*)"',body):
            src=m.group(1).encode().decode('unicode_escape')
            ok_svg(f'block{n} rust {src[:20]!r}',src)
# example files named
names=sorted(set(re.findall(r'`((?:examples/)?[\w-]+\.(?:mmd|rs|json))`',new)))
print(names)
for nm in names:
    base=os.path.basename(nm)
    p=WT+'/examples/'+base
    if not os.path.exists(p): res.append((f'file {nm} exists',False,0,'')); continue
    if base.endswith('.mmd'):
        if base.startswith('advance'):
            rc,out,err=run(['--advance-text',p])
        else:
            rc,out,err=run([p])
        res.append((f'example {base}',rc==0 and out.startswith('<svg'),rc,err[:160]))
    else: res.append((f'file {nm} exists',True,0,''))
# CLI lines
rc,out,err=run(['examples/demo.mmd','-o',OUT+'/demo.svg']); res.append(('cli -o',rc==0 and os.path.getsize(OUT+'/demo.svg')>100,rc,err))
rc,out,err=run([],inp=open(WT+'/examples/lr.mmd').read()); res.append(('cli pipe',rc==0 and out.startswith('<svg'),rc,err))
rc,out,err=run(['examples/shapes.mmd','--compact','600']); res.append(('cli compact',rc==0 and out.startswith('<svg'),rc,err))
rc,out,err=run(['examples/demo.mmd','--transparent']); res.append(('cli transparent',rc==0 and out.startswith('<svg'),rc,err))
for t in ['flowmaid','default','neutral','dark','forest']:
    rc,out,err=run(['examples/demo.mmd','--theme',t]); res.append((f'cli theme {t}',rc==0 and out.startswith('<svg') and not err.strip(),rc,err))
rc,out,err=run(['examples/demo.mmd','--theme','nosuch']); res.append(('cli theme unknown warns, still renders',rc==0 and out.startswith('<svg') and bool(err.strip()),rc,err))
for nm,good,rc,err in res:
    print('OK  ' if good else 'FAIL',nm,rc,err)
