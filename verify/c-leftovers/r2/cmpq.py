import csv,collections,sys
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/c-leftovers/r2/'
def load(p):
    d={}
    for row in csv.reader(open(p),delimiter='\t'):
        if row[0]=='file': continue
        if len(row)<12: d[(row[0],'PANIC')]=None; continue
        d[(row[0],row[1])]=row[2:]
    return d
name=sys.argv[1]
a=load(V+name+'-pre.tsv'); b=load(V+name+'-now.tsv')
print(len(a),len(b),[k for k in list(a)+list(b) if k[1]=='PANIC'][:5])
cols=['curves','cross','loop_cross','through','loop_node','chip_edge','chip_chip','chip_node']
for modes in [('scene','route'),('dragA','dragB','dragC','dragD')]:
    inc=collections.Counter(); dec=collections.Counter(); tot=collections.Counter(); ex=collections.defaultdict(list); rows=0;chg=0
    for k in a:
        if k not in b or k[1] not in modes or a[k] is None or b[k] is None: continue
        rows+=1
        if a[k]!=b[k]: chg+=1
        x=[int(v) for v in a[k][1:9]]; y=[int(v) for v in b[k][1:9]]
        for c,xx,yy in zip(cols,x,y):
            if yy>xx: inc[c]+=1; tot[c]+=yy-xx; ex[c].append((k[0].split('/')[-1],k[1],xx,yy))
            elif yy<xx: dec[c]+=1; tot[c]+=yy-xx
    print(modes,'rows',rows,'changed',chg)
    for c in cols: print('  ',c,'worse rows',inc[c],'better rows',dec[c],'net',tot[c], ex[c][:10])
