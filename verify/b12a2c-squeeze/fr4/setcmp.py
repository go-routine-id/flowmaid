import sys,csv
def load(p):
    r=list(csv.reader(open(p),delimiter='\t')); h=r[0]
    return h,{(x[0],x[1]):dict(zip(h,x)) for x in r[1:] if x and x[0] not in('file','TOTAL')}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
cols=['defects','x_drawn','lateral_x','fan_x','edge_chip','side_ends','end_kink','overlap_pairs','edge_node_x','lbl_crowd','chip_crowd','sib_stagger','tight_len','band_pairs']
worse=[];diff=[]
for k in sorted(a):
    if k not in b: worse.append((k,'MISSING')); continue
    d=[]
    for c in cols+['min_edge_gap','chip_gap_min']:
        x,y=a[k].get(c),b[k].get(c)
        if x!=y: d.append((c,x,y))
    if d: diff.append((k,d))
for k,d in diff: print(k[0],k[1],d)
