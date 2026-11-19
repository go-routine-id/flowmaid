import sys,csv
# gated rows: bends, bends_max (+edge), straight_max (+edge), lane_gap_min, lanes_max for the inputs with an edge-shape rule
B=sys.argv[1]
def load(f):
    r=list(csv.reader(open(f),delimiter='\t')); h=r[0]; return {x[0]:dict(zip(h,x)) for x in r[1:]}
names=['infra-overview','infra-overview-v2','perm-a-edges-reversed','perm-b-subgraphs-reversed','perm-c-actors-reversed','perm-d-edges-rotated','perm-e-one-more-inner-edge','blocks-overview','blocks-overview-edges-reversed','blocks-overview-edges-rotated','blocks-overview-actors-reversed','blocks-overview-subgraphs-reversed']
cols=sys.argv[2].split(',')
for t,lab in [('038','0.38'),('start','start'),('mermaid','mermaid'),('fr4','7122a53')]:
    o=load(f'{B}/data/orig-{t}.tsv'); n=load(f'{B}/data/fix-{t}.tsv')
    print('==',lab)
    for nm in names:
        print('  %-36s'%nm, '  '.join(f"{c} {o[nm][c]}->{n[nm][c]}" for c in cols))
