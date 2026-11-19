#!/usr/bin/env bash
# q1.sh <dir>: short static harness rows
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12; Q=${Q:-$B/target-on/release/examples/quality}
$Q $1 | python3 -c "
import sys,csv
r=list(csv.reader(sys.stdin,delimiter='\t')); h=r[0]
for x in r[1:]:
  d=dict(zip(h,x))
  if d.get('mode')=='static' and d['file']!='TOTAL': print(f\"{d['file'][:24]:24s} x {d['x_drawn']:>3} fan {d['fan_x']:>2} gap {d['min_edge_gap']:>6} def {d['defects']} lat {d.get('lateral_x')} side {d.get('side_ends')} cmax {d.get('compact_max')} sst {d.get('sib_stagger')} W {d['width']} H {d['height']} ms {d['scene_ms']}\")
"
