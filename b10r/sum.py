import csv,sys
D='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b10r/out/'
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t'));h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h) and r[0]!='TOTAL'}
a,b=sys.argv[1],sys.argv[2]
sets=sys.argv[3].split(',') if len(sys.argv)>3 else ['corp','extra','st','fc']
cols=['x_drawn','edge_node_x','edge_frame_x','defects','lbl_edge','idiom_scubic','idiom_bow','idiom_spline','border_runs','overlap_pairs']
for s in sets:
    A=load(D+a+'/'+s+'.tsv');B=load(D+b+'/'+s+'.tsv')
    for mode in ['static','routed']:
        ks=[k for k in A if k[1]==mode and k in B]
        out=[]
        for c in cols:
            sa=sum(float(A[k][c]) for k in ks); sb=sum(float(B[k][c]) for k in ks)
            out.append(f"{c} {sa:g}->{sb:g}")
        worse=sum(1 for k in ks if float(B[k]['x_drawn'])>float(A[k]['x_drawn']))
        better=sum(1 for k in ks if float(B[k]['x_drawn'])<float(A[k]['x_drawn']))
        print(f"{s:5} {mode:6} n={len(ks)} worse={worse} better={better} | "+'  '.join(out))
if len(sys.argv)>4:
    s=sys.argv[4]; A=load(D+a+'/'+s+'.tsv');B=load(D+b+'/'+s+'.tsv')
    d=sorted(((float(B[k]['x_drawn'])-float(A[k]['x_drawn'])),k) for k in A if k in B)
    for v,k in d[-25:][::-1]:
        if v>0: print(v,k,A[k]['x_drawn'],B[k]['x_drawn'])
