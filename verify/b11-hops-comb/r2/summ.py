import re,sys
keys=['scenes','hops','holes','probes_a','probes_b','a_none','a_other','b_phantom','b_missed','hop_shape','hop_dev35','hop_dev60','hop_twin','hop_foot_x','hop_no_crossing','hop_node','hop_label','hop_frame','hop_ink','hop_head','hop_gap_near','sharp','geom_arc','idx','nondet','cross_total','cross_hop','cross_gap','cross_plain']
for p in sys.argv[1:]:
    tot=None; files=0; fails=[]
    for l in open(p):
        if l.startswith('  FAIL'):
            if 'not flanked' not in l: fails.append(l.rstrip()[:300])
            continue
        if l.startswith('TOTAL'):
            tot=dict(re.findall(r'(\w+): ([-\d.]+)',l))
        else: files+=1
    print('==',p,'files',files)
    if tot: print('  '+' '.join(f"{k}={tot[k]}" for k in keys if k in tot), 'devmax=',tot.get('hop_dev_max'), 'off_path=',tot.get('off_path'))
    else: print('  (running)')
    kinds={}
    for f in fails:
        k=re.sub(r'[-\d.]+','#',f.split(': ',1)[1])[:60]
        kinds.setdefault(k,[]).append(f)
    for k,v in kinds.items():
        print('  ',len(v),'x',k); 
        for f in v[:6]: print('      ',f)
