import sys,csv,glob,os
H='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r6/h'
def load(p):
    rows={}
    if not os.path.exists(p): return rows
    for r in csv.DictReader(open(p),delimiter='\t'):
        if r['file']=='TOTAL': continue
        rows[(r['file'],r['mode'])]=r
    return rows
mode=sys.argv[1] if len(sys.argv)>1 else 'static'
cols=(sys.argv[2] if len(sys.argv)>2 else 'x_drawn,fan_x,lateral_x,overlap_pairs,edge_frame_pairs,defects').split(',')
sets=(sys.argv[3] if len(sys.argv)>3 else 'quality,blocks,notes,fans,xin,shuf,fam,ex').split(',')
hard='edge_node_x,shared_ports,arrow_on_start,wiggles,frame_side,node_overlap,node_frame,frame_overlap,own_frame_x'.split(',')
only_diff=os.environ.get('DIFF')=='1'
tot={c:[0,0] for c in cols}
for s in sets:
    a=load(f'{H}/off-{s}.tsv'); b=load(f'{H}/on-{s}.tsv')
    for k in sorted(b):
        if k[1]!=mode: continue
        pa=a.get(k,{})
        hz_a=sum(float(pa.get(h,0) or 0) for h in hard); hz_b=sum(float(b[k].get(h,0) or 0) for h in hard)
        vals=[(c,pa.get(c,'?'),b[k].get(c,'?')) for c in cols]
        for c,x,y in vals:
            try: tot[c][0]+=float(x); tot[c][1]+=float(y)
            except: pass
        if only_diff and all(x==y for _,x,y in vals) and hz_a==hz_b: continue
        print(f"{s:7s} {k[0]:34s} "+'  '.join(f"{c}={x}->{y}" for c,x,y in vals)+f"  hz={hz_a:g}->{hz_b:g}  bm={b[k].get('block_mode','?')}")
print('TOTAL', {c:(round(v[0],2),round(v[1],2)) for c,v in tot.items()})
