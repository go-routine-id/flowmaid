import csv,sys
def load(f):
    rows=list(csv.reader(open(f),delimiter='\t'))
    h=rows[0]
    return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:]}
HARD=['edge_node_x','shared_ports','arrow_on_start','wiggles','frame_side','node_overlap','node_frame','frame_overlap','own_frame_x']
cols=['defects','edge_frame_pairs','overlap_pairs','x_drawn','fan_x']
mode=sys.argv[4] if len(sys.argv)>4 else 'static'
_,a=load(sys.argv[1]); _,b=load(sys.argv[2]); _,c=load(sys.argv[3])
def v(d,k): 
    try: return int(float(d[k]))
    except: return d.get(k)
tot={x:[0,0,0] for x in cols+['hard']}
print(f"{'file':40s} " + ' '.join(f"{x[:9]:>14s}" for x in cols+['hard']) + "  (start/prev/now)")
for key in sorted(a):
    if key[1]!=mode or key not in b or key not in c: continue
    vals=[]
    flag=''
    for x in cols+['hard']:
        t=[ (sum(v(d[key],h) for h in HARD) if x=='hard' else v(d[key],x)) for d in (a,b,c)]
        for i in range(3): tot[x][i]+=t[i]
        vals.append(t)
        if t[2]!=t[1]: flag+='*'
    bm=c[key].get('block_mode','?')
    print(f"{key[0]:40s} " + ' '.join(f"{'/'.join(map(str,t)):>14s}" for t in vals) + f"  bm={bm} {flag}")
print('TOTAL', {k:v for k,v in tot.items()})
