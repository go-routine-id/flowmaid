#!/usr/bin/env python3
# Compare harness rows blocks-off vs blocks-on: hard zeros, fan_x, x_drawn, edge_frame_pairs, overlap, defects.
import csv,sys
HZ=['edge_node_x','shared_ports','arrow_on_start','wiggles','frame_side','node_overlap','node_frame','frame_overlap','own_frame_x']
def load(p):
    rows=list(csv.reader(open(p),delimiter='\t')); h=rows[0]
    return {(r[0],r[1]):dict(zip(h,r)) for r in rows[1:] if len(r)==len(h)}
off=load(sys.argv[1]); on=load(sys.argv[2]); mode=sys.argv[3] if len(sys.argv)>3 else 'static'
only_diff = len(sys.argv)>4
tot={'hz_off':0,'hz_on':0,'fan_off':0,'fan_on':0,'x_off':0,'x_on':0,'efp_off':0,'efp_on':0}
print('file | block | hz off->on (which) | fan_x | x_drawn | edge_frame_pairs | overlap_pairs | defects')
for k in sorted(on):
    if k[1]!=mode or k[0]=='TOTAL': continue
    a=off.get(k); b=on[k]
    if a is None: continue
    hz=lambda r: sum(int(float(r[c])) for c in HZ)
    which=' '.join(f'{c}={b[c]}' for c in HZ if int(float(b[c])))
    tot['hz_off']+=hz(a); tot['hz_on']+=hz(b)
    tot['fan_off']+=int(a['fan_x']); tot['fan_on']+=int(b['fan_x'])
    tot['x_off']+=int(a['x_drawn']); tot['x_on']+=int(b['x_drawn'])
    tot['efp_off']+=int(a['edge_frame_pairs']); tot['efp_on']+=int(b['edge_frame_pairs'])
    line=f"{k[0]:36s} bm={b['block_mode']} hz {hz(a)}->{hz(b)} [{which}] fan {a['fan_x']}->{b['fan_x']} x {a['x_drawn']}->{b['x_drawn']} efp {a['edge_frame_pairs']}->{b['edge_frame_pairs']} ovl {a['overlap_pairs']}->{b['overlap_pairs']} def {a['defects']}->{b['defects']}"
    worse = hz(b)>0 or int(b['fan_x'])>int(a['fan_x']) or int(b['edge_frame_pairs'])>int(a['edge_frame_pairs']) or int(b['overlap_pairs'])>int(a['overlap_pairs']) or int(b['defects'])>int(a['defects']) or int(b['x_drawn'])>int(a['x_drawn'])
    if not only_diff or worse: print(line)
print(tot)
