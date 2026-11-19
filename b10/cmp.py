import sys,csv
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'))
    h=rows[0]; return h,{(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
h,a=load(S+'/runs/'+sys.argv[1]+'/perf.tsv'); _,b=load(S+'/runs/'+sys.argv[2]+'/perf.tsv')
cols=sys.argv[3].split(',') if len(sys.argv)>3 else ['defects','edge_node_x','edge_frame_x','edge_edge_x','overlap_pairs','tight_pairs','x_drawn','idiom_scubic','idiom_spline','idiom_bow','idiom_ortho','end_kink','band_pairs','lbl_edge','lbl_lbl','lbl_node','width','height']
print('file mode '+' '.join(cols))
for k in sorted(a):
    ra,rb=a[k],b.get(k,{})
    out=[]
    for c in cols:
        x,y=ra.get(c,'?'),rb.get(c,'?')
        out.append(f"{c}={x}" if x==y else f"{c}={x}->{y}")
    print(k[0],k[1],' '.join(out))
