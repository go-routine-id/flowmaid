import csv,sys
# hc.py cols file... : static rows, side by side per file
cols=sys.argv[1].split(','); files=sys.argv[2:]
tabs=[]
for f in files:
    t={}
    for r in csv.DictReader(open(f),delimiter='\t'):
        if r.get('mode')=='static': t[r['file']]=r
    tabs.append(t)
names=sorted(set().union(*[t.keys() for t in tabs]))
for n in names:
    vals=[]
    for t in tabs:
        r=t.get(n); vals.append(' '.join(f"{c}={r.get(c,'?')}" for c in cols) if r else '-')
    if len(set(vals))>1 or len(files)==1: print(f"{n[:28]:28s}", ' | '.join(vals))
