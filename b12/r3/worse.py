import csv,sys
def load(p):
    rows=list(csv.DictReader(open(p),delimiter='\t'))
    return {(r['file'],r['mode']):r for r in rows}
B=load('../tools-a2b/base-157117e/dirs/dirs.tsv')
V=load(f'runs/{sys.argv[1]}/dirs/dirs.tsv')
cols=['defects','x_drawn','lateral_x','fan_x','edge_chip','side_ends']
n=0
for k in sorted(B):
    if k[1]!='routed' or k[0]=='TOTAL': continue
    b,v=B[k],V.get(k)
    if v is None: continue
    w=[f"{c} {b[c]}->{v[c]}" for c in cols if float(v[c])>float(b[c])]
    if float(v['min_edge_gap'])<float(b['min_edge_gap'])-0.5: w.append(f"gap {b['min_edge_gap']}->{v['min_edge_gap']}")
    better=[c for c in cols if float(v[c])<float(b[c])]
    if w:
        n+=1; print(k[0].ljust(40),'; '.join(w), ' | better:',','.join(better))
print(n,'copies worse')
