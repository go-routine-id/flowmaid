import sys,csv
FIX=['back-edge','bench-75','dense-lr','dense-td','fanout-frames','wide-labels']
MAXC={'end_tilt_max','end_gap_max','overlap_max','xgap_max','hug_max'}
def load(p):
    r=list(csv.reader(open(p),delimiter='\t')); h=r[0]
    return {(x[0],x[1]):dict(zip(h,x)) for x in r[1:]}
cols='border_len hug_len overlap_len overlap_max tight_len edge_chip xgap_max lbl_edge lbl_node lbl_lbl end_tilt end_tilt_max end_gaps lat_wiggles lat_backtrack own_frame_x band_pairs band_len idiom_scubic idiom_spline idiom_bow x_drawn end_kink corner_touch min_node_gap end_cross'.split()
ds=[load(p) for p in sys.argv[1:]]
for mode in ['static','routed']:
    for c in cols:
        vals=[]
        for d in ds:
            v=[float(d[(f,mode)][c]) for f in FIX if (f,mode) in d]
            if c=='min_node_gap': vals.append(min(v))
            elif c in MAXC: vals.append(max(v))
            else: vals.append(sum(v))
        flag='' if len(set(round(x,1) for x in vals))==1 else '  *'
        print(mode,c,' '.join(f'{x:.1f}' for x in vals),flag)
