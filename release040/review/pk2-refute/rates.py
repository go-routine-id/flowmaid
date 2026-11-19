import csv, re, sys, os
from collections import Counter
V=sys.argv[1]; R=sys.argv[2]
rel=re.compile(r'^\s*([\w.`~<>-]*?[\w`>])\s*(?:"[^"]*")?\s*(<\|--|--\|>|\*--|--\*|o--|--o|<-->|-->|<--|--|\.\.>|<\.\.|\.\.\|>|<\|\.\.|\*\.\.|\.\.\*|o\.\.|\.\.o|\.\.)\s*(?:"[^"]*")?\s*([\w`][\w.`~<>]*)')
def maxloops(path):
    p=path if os.path.isabs(path) else os.path.join(V,path)
    c=Counter()
    try: txt=open(p,encoding='utf-8',errors='replace').read()
    except OSError: return None,None
    for ln in txt.splitlines():
        m=rel.match(ln)
        if m and m.group(1).strip('`')==m.group(3).strip('`'): c[m.group(1).strip('`')]+=1
    return (max(c.values()) if c else 0), sum(c.values())
def load(p):
    rows={}
    for r in csv.DictReader(open(p),delimiter='\t'):
        if r.get('frames') is None: continue
        rows[r['file']]={k:int(r[k]) for k in ('loops','frames','flips','flipframes','inplace_diff','inplace_loopdiff')}
    return rows
for name,label in [('cfz2','1,200 random'),('cfz','500 loop-heavy'),('lp','432 sweep'),('bulk','894 corpus')]:
    pre=load(f'{R}/fl-{name}-hp.tsv'); now=load(f'{R}/fl-{name}-hn.tsv')
    assert pre.keys()==now.keys()
    fp=sum(r['frames'] for r in pre.values())
    print(f"{label}: files {len(pre)} (with loops {sum(1 for r in pre.values() if r['frames'])}), frames {fp}: "
          f"{100*sum(r['flipframes'] for r in pre.values())/fp:.2f}% -> {100*sum(r['flipframes'] for r in now.values())/fp:.2f}%")
    ml={}; bad=0
    for k,r in pre.items():
        m,t=maxloops(k)
        if t!=r['loops']: bad+=1
        ml[k]=m
    print(f"   parser check: loop count disagrees with probe in {bad} files")
    for lab,cond in [('no class with >=4 loops',lambda m:m is not None and m<4),('some class with >=4 loops',lambda m:m is not None and m>=4)]:
        ks=[k for k in pre if pre[k]['frames'] and cond(ml[k])]
        f=sum(pre[k]['frames'] for k in ks)
        if f: print(f"   {lab}: {len(ks)} files, {100*sum(pre[k]['flipframes'] for k in ks)/f:.2f}% -> {100*sum(now[k]['flipframes'] for k in ks)/f:.2f}%")
        else: print(f"   {lab}: 0 files")
