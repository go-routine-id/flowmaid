#!/usr/bin/env python3
"""Per frame of a 0.38 scene (structdump JSON): frame breadth extent minus the member-only box
(members + SUB_PAD 14 each side). TD: breadth = x. LR: breadth = y (title strip adds to top)."""
import json, sys, subprocess
B='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12'
for p in sys.argv[1:]:
    out = subprocess.run([B+'/metrics/bin/structdump-038', p], capture_output=True, text=True).stdout
    st = json.loads(out)
    sc = st.get('scene')
    if not sc: print(p, 'no scene', list(st.keys())); continue
    ns = st['nodes']; subs = st['subgraphs']
    horiz = st.get('direction') in ('LR','RL')
    def inside(v, si):
        s = ns[v]['sub']
        while s is not None:
            if s == si: return True
            s = subs[s]['parent']
        return False
    nodes = sc['nodes']
    for c in sc['clusters']:
        si = next(i for i,s in enumerate(subs) if s['id']==c['id'])
        mem=[nodes[v] for v in range(len(ns)) if inside(v,si)]
        if not mem: continue
        if not horiz:
            lo=min(n['x'] for n in mem)-14; hi=max(n['x']+n['w'] for n in mem)+14
            f0=c['x']; f1=c['x']+c['w']
        else:
            lo=min(n['y'] for n in mem)-14; hi=max(n['y']+n['h'] for n in mem)+14
            f0=c['y']; f1=c['y']+c['h']
        print(f"{p.split('/')[-1]:28s} {c['id']:8s} frame [{f0:.1f},{f1:.1f}] members+pad [{lo:.1f},{hi:.1f}] widened {lo-f0:+.1f} / {f1-hi:+.1f}")
