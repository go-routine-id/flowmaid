import sys,csv,glob,os
# hcmp.py <labelA> <labelB> [cols]: static (and routed with R=1) rows that differ
W=os.path.dirname(os.path.abspath(__file__))+'/h'
a,b=sys.argv[1],sys.argv[2]
cols=(sys.argv[3] if len(sys.argv)>3 else 'x_drawn,fan_x,overlap_pairs,min_edge_gap,edge_node_x,shared_ports,arrow_on_start,frame_side,node_frame,own_frame_x,lateral_x,edge_chip,wiggles,defects,width,height').split(',')
modes=['static','routed'] if os.environ.get('R') else ['static']
def load(p):
    rows={}
    try:
        with open(p) as f:
            r=csv.reader(f,delimiter='\t'); h=next(r)
            for x in r:
                if x[0]=='TOTAL' or x[1] not in modes: continue
                rows[(x[0],x[1])]=dict(zip(h,x))
    except Exception: pass
    return rows
n=0
for pa in sorted(glob.glob(f'{W}/{a}-*.tsv')):
    s=pa[len(f'{W}/{a}-'):-4]
    A=load(pa); B=load(f'{W}/{b}-{s}.tsv')
    for k in sorted(set(A)|set(B)):
        ra,rb=A.get(k,{}),B.get(k,{})
        d=[c for c in cols if ra.get(c)!=rb.get(c)]
        if d:
            n+=1
            print(f"{s:8s} {k[0]:34s} {k[1][:1]} "+'  '.join(f"{c}={ra.get(c,'?')}->{rb.get(c,'?')}" for c in d))
print('rows differing:',n)
