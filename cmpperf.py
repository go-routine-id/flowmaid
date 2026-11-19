import csv,sys
def load(p):
    d={}
    for r in csv.DictReader(open(p),delimiter='\t'):
        d[(r['file'],r['mode'])]=r
    return d
labs=sys.argv[1].split(',')
files=sys.argv[2].split(',')
cols=sys.argv[3].split(',') if len(sys.argv)>3 else ['defects','min_node_gap','min_edge_gap','edge_frame_x','edge_edge_x','overlap_pairs','overlap_len','border_len','tight_len','hug_len','lbl_edge','corner_touch','end_cross','band_pairs','band_len','x_drawn','end_kink','drag_ms','drag_mean','drag_over','route_ms','scene_ms']
runs={l:load(f'/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/runs/{l}/perf.tsv') for l in labs}
print('%-16s'%'', '  '.join('%12s'%l[-12:] for l in labs))
for f in files:
    for m in ['static','routed']:
        print(f,m)
        for c in cols:
            vals=[runs[l].get((f,m),{}).get(c,'-') for l in labs]
            mark = '' if len(set(vals))==1 else ' *'
            print('  %-14s'%c, '  '.join('%12s'%v for v in vals), mark)
