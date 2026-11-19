import csv,sys
def load(p):
    rows=list(csv.DictReader(open(p),delimiter='\t'))
    return {(r['file'],r['mode']):r for r in rows}
B=load('../tools-a2b/base-157117e/dirs/dirs.tsv')
for run in sys.argv[1:]:
    V=load(f'runs/{run}/dirs/dirs.tsv')
    print(run)
    for f in ['quality.dense-td-TD','quality.dense-td-BT','quality.dense-td-LR','quality.dense-td-RL','quality.dense-lr-TD','quality.dense-lr-LR','quality.bench-75-TD','quality.bench-75-BT','quality.bench-75-LR','quality.bench-75-RL','blocks.blocks-overview-TD','blocks.blocks-overview-BT','blocks.blocks-overview-LR']:
        b=B[(f,'routed')]; v=V[(f,'routed')]
        print('  ',f.ljust(26),' '.join(f"{c}:{b[c]}->{v[c]}" for c in ['defects','x_drawn','fan_x','lateral_x','side_ends','min_edge_gap','route_ms']))
