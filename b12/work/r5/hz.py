#!/usr/bin/env python3
"""hz.py <quality-bin-A> <quality-bin-B> <dirs...>: static hard zeros / crossings / fan_x side by side (A=before, B=after)."""
import subprocess, sys, csv, io
HZ = ['edge_node_x','shared_ports','arrow_on_start','wiggles','frame_side','node_overlap','node_frame','frame_overlap','own_frame_x']
def run(q, d):
    out = subprocess.run([q, d], capture_output=True, text=True)
    return {r['file']: r for r in csv.DictReader(io.StringIO(out.stdout), delimiter='\t') if r['mode']=='static' and r['file']!='TOTAL'}
a_bin, b_bin = sys.argv[1], sys.argv[2]
tot = {}
for d in sys.argv[3:]:
    A, B = run(a_bin, d), run(b_bin, d)
    for f in sorted(B):
        a, b = A.get(f), B[f]
        if not a: continue
        ha = sum(float(a[c]) for c in HZ); hb = sum(float(b[c]) for c in HZ)
        vals = [(c, float(a[c]), float(b[c])) for c in ['x_drawn','fan_x','edge_frame_pairs','overlap_pairs','defects']]
        for c, x, y in [('hz', ha, hb)] + vals:
            t = tot.setdefault(c, [0,0]); t[0]+=x; t[1]+=y
        diff = [f'hz {ha:g}->{hb:g} ' + ' '.join(f'{c[:5]}:{x:g}' for c in HZ for x in [float(b[c])] if x) ] if hb != ha else []
        diff += [f'{c} {x:g}->{y:g}' for c, x, y in vals if x != y]
        if diff: print(f'{d.split("/")[-1]}/{f}: ' + '; '.join(diff))
print('TOTAL', {c: (round(v[0],1), round(v[1],1)) for c, v in tot.items()})
