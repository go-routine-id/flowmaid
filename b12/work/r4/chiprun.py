import re,sys,subprocess,os
B=os.environ['B']
for f in sys.argv[1:]:
    out=subprocess.run([B+'/target-dbg/release/flowmaid',f,'-o','/dev/null'],env=dict(os.environ,FM_XXDBG='1',FM_XXDBG7='1'),capture_output=True,text=True).stderr
    # first order dump only
    layers=[]; seen=False
    for line in out.splitlines():
        if line.startswith('XXDBG order'):
            if seen: break
            seen=True; continue
        m=re.match(r'XXDBG L(\d+):(.*)',line)
        if m and seen: layers.append(m.group(2))
    first={}  # path -> first layer
    onrow={}  # (edge) -> list of (path, layer)
    for l,txt in enumerate(layers):
        cur=''
        for tok in re.findall(r'\|[^|]*\||\([^)]*\)|\S+',txt):
            if tok.startswith('|'): cur=tok.strip('|'); 
            if cur and cur not in first: first[cur]=l
            if tok.startswith('('):
                onrow.setdefault(tok.strip('()').replace('>','->'),[]).append((cur,l))
    bad=[re.match(r'XXDBG bad class \d (\S+) .*chip: (\d+)',x) for x in out.splitlines() if 'XXDBG bad' in x]
    chip=[m.group(1) for m in bad if m and m.group(2)!='0']
    res=[]
    for e in chip:
        runs=[(p,l) for p,l in onrow.get(e,[]) if p and first.get(p)==l]
        res.append(e+(' RUN@'+','.join(p for p,_ in runs) if runs else ' direct'))
    print(os.path.basename(f), len(chip), res)
