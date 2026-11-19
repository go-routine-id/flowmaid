# offenders.py <outdir> <name...>: per diagram, the labels / edges behind the label and edge-shape rows
import subprocess, sys, re
M='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/metrics'
out=sys.argv[1]
for name in sys.argv[2:]:
    r=subprocess.run(['/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/work/a2i/tools/bm-gapedge','--struct-dir',M+'/../local/struct','--detail',f'{out}/{name}.svg'],capture_output=True,text=True)
    lines=r.stderr.split('\n')
    sec=None; hdr=None; edges=[]; labels=[]
    for l in lines:
        if l.startswith(' edges:'): sec='e'; hdr=None; continue
        if l.startswith(' labels:'): sec='l'; hdr=None; continue
        if l.startswith(' ') and not l.startswith('  '): sec=None; continue
        if sec and l.startswith('  '):
            f=l.split()
            if hdr is None: hdr=f; continue
            if sec=='e': edges.append(dict(zip(hdr,f)))
            else:
                # text has spaces: fields after text are the last len(hdr)-2 tokens
                n=len(hdr)-2
                d=dict(zip(hdr[2:],f[-n:])); d['edge']=f[0]; d['text']=' '.join(f[1:-n]); labels.append(d)
    print('==',name)
    for d in labels:
        g=float(d['gap']); 
        flags=[]
        if g<12: flags.append(f"crowd gap {g} to {d['gap_edge']}")
        if d['struck']!='0': flags.append('struck by '+d['struck_by'])
        if d['in_end']!='-': flags.append('in_end '+d['in_end'])
        if float(d['own_gap'])>1: flags.append('own_gap '+d['own_gap'])
        if flags: print(f"  label {d['edge']} '{d['text'][:30]}' end_px {d['end_px']}: "+', '.join(flags))
    if edges:
        e=max(edges,key=lambda d:int(d['bends'])); print(f"  bends_max {e['bends']} {e['edge']}; bends total {sum(int(d['bends']) for d in edges)}")
        de=[d for d in edges if d['detour']!='-']
        if de: e=max(de,key=lambda d:float(d['detour'])); print(f"  detour_max {e['detour']} {e['edge']}")
        e=max(edges,key=lambda d:float(d['straight_max'])); print(f"  straight_max {e['straight_max']} {e['edge']}")
        lx=[d['edge'] for d in edges if d['lateral_x']!='0']; print(f"  lateral_x edges: {' '.join(lx)}")
