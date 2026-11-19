import csv,collections,sys
def load(p):
    return {(r[0],r[1]):r[2:] for r in csv.reader(open(p),delimiter='\t') if r[0]!='file' and len(r)>3}
for s in sys.argv[1:]:
    a=load(f'r3/le-{s}-pre.tsv'); b=load(f'r3/le-{s}-now.tsv')
    print('==',s,len(a),len(b))
    for modes in [('scene',),('dragA','dragB','dragC','dragD')]:
        st=collections.defaultdict(lambda:[0,0,0,0,0,0.0,0.0])  # by maxcls: rows, worse, better, pre>6, now>6, premax, nowmax
        ex=[]
        for k in a:
            if k[1] not in modes or k not in b: continue
            ma,mb=float(a[k][2]),float(b[k][2]); mc=int(a[k][4])
            if mc==0: continue
            g=st[min(mc,9)]; g[0]+=1
            if mb>ma+0.5: g[1]+=1; ex.append((mb-ma,k[0].split('/')[-1],k[1],ma,mb,mc))
            elif mb<ma-0.5: g[2]+=1
            if ma>6: g[3]+=1
            if mb>6: g[4]+=1
            g[5]=max(g[5],ma); g[6]=max(g[6],mb)
        print(' ',modes)
        for mc in sorted(st): g=st[mc]; print(f'    loops/class {mc}: rows {g[0]} worse {g[1]} better {g[2]} | off>6px pre {g[3]} now {g[4]} | max pre {g[5]} now {g[6]}')
        for e in sorted(ex,reverse=True)[:5]: print('     ',e)
