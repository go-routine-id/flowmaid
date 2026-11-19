import sys,csv,os
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure'
def load(p):
    rows={}
    if not os.path.exists(p): return rows
    for r in csv.DictReader(open(p),delimiter='\t'):
        if r['file']=='TOTAL': continue
        rows[(r['file'],r['mode'])]=r
    return rows
mode=sys.argv[1]
cols=sys.argv[2].split(',')
sets=sys.argv[3].split(',')
A=sys.argv[4] if len(sys.argv)>4 else 'r7/h/pre'   # reference
Bp=sys.argv[5] if len(sys.argv)>5 else 'r6/h/on'   # previous round
C=sys.argv[6] if len(sys.argv)>6 else 'r7/h/on'    # now
hard='edge_node_x,shared_ports,arrow_on_start,wiggles,frame_side,node_overlap,node_frame,frame_overlap,own_frame_x'.split(',')
only=os.environ.get('DIFF')
def hz(r): return sum(float(r.get(h,0) or 0) for h in hard) if r else float('nan')
for s in sets:
    a=load(f'{V}/{A}-{s}.tsv'); b=load(f'{V}/{Bp}-{s}.tsv'); c=load(f'{V}/{C}-{s}.tsv')
    for k in sorted(c):
        if k[1]!=mode: continue
        ra,rb,rc=a.get(k,{}),b.get(k,{}),c[k]
        vals=[(col,ra.get(col,'?'),rb.get(col,'?'),rc.get(col,'?')) for col in cols]
        h=(hz(ra),hz(rb),hz(rc))
        if only=='prev' and all(y==z for _,_,y,z in vals) and h[1]==h[2]: continue
        print(f"{s:7s} {k[0]:34s} "+'  '.join(f"{col}={x}|{y}|{z}" for col,x,y,z in vals)+f"  hz={h[0]:g}|{h[1]:g}|{h[2]:g} bm={rc.get('block_mode','?')}")
