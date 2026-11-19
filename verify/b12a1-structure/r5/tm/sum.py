import csv,glob,sys,collections
d='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5/tm/'
res=collections.defaultdict(lambda: collections.defaultdict(list))
for f in sorted(glob.glob(d+'*-*.tsv')):
    w=f.split('/')[-1].split('-')[0]
    r=csv.reader(open(f),delimiter='\t'); h=next(r)
    for x in r:
        row=dict(zip(h,x))
        if row['file']=='TOTAL': continue
        if row['mode']=='static': res[row['file']][(w,'scene')].append(float(row['scene_ms']))
        else:
            res[row['file']][(w,'route')].append(float(row['route_ms'])); res[row['file']][(w,'drag')].append(float(row['drag_ms']))
for fn in sorted(res):
    s=fn.ljust(20)
    for m in ['scene','route','drag']:
        s+=' | '+m+' '+' '.join(f"{w}={min(res[fn][(w,m)]) if res[fn][(w,m)] else float('nan'):.2f}" for w in ['pre','off','on'])
    print(s)
