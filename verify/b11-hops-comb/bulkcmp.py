import sys,csv
V=sys.argv[1]
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return h,[dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)]
skip={'route_ms','scene_ms','drag_ms','drag_mean','drag_over','deterministic','mode','ink','idiom_ortho','xhops','min_gap','min_gap_f','xgap_max','xnotches','dash_edges'}
for name in sys.argv[2:]:
    hh,a=load(f'{V}/{name}_pre.tsv'); hn,b=load(f'{V}/{name}_now.tsv')
    ka={(r[hh[0]],r['mode']):r for r in a}
    cols=[c for c in hh[2:] if c not in skip]
    print('==',name,len(a)//2,'files; cols:',len(cols))
    for m in ['static','routed']:
        ta=ka[('TOTAL',m)]; tb=[r for r in b if r[hn[0]]=='TOTAL' and r['mode']==m][0]
        print(' TOTAL',m,' '.join(f"{c}:{ta.get(c,'-')}->{tb.get(c,'-')}" for c in hn[2:] if ta.get(c)!=tb.get(c) and c not in ('route_ms','scene_ms')))
    worse={};better={}
    files_worse=set(); nondet=[]
    for r in b:
        k=(r[hn[0]],r['mode']); o=ka.get(k)
        if r.get('deterministic') not in ('true','1',None) and k[0]!='TOTAL': nondet.append(k)
        if not o or k[0]=='TOTAL': continue
        for c in cols:
            try: d=float(r[c])-float(o[c])
            except: continue
            if c in ('min_gap',): continue
            if d>1e-9: worse.setdefault(c,[]).append((k,o[c],r[c])); files_worse.add(k)
            elif d<-1e-9: better.setdefault(c,[]).append((k,o[c],r[c]))
    print(' nondeterministic:',nondet[:5])
    for c in cols:
        if c in worse or c in better:
            print(f"  {c:18s} worse cells {len(worse.get(c,[])):4d}  better cells {len(better.get(c,[])):4d}")
    for c in ['defects','edge_node_pairs','edge_frame_pairs','overlap_pairs','tight_pairs','lbl_node','lbl_lbl','lbl_edge','lbl_arrow','shared_ports','x_drawn','edge_edge_x','own_frame_x','ink_out','end_kink','end_gaps','fan_x','fan_uneven','wiggles','lat_wiggles','hug_runs','border_runs']:
        for w in worse.get(c,[])[:12]: print('   WORSE',c,w)
