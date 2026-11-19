#!/usr/bin/env python3
"""detach_all.py <out.json> <svg...>: every label's detach (chip centre to own path) per svg, via blockmetrics --detail."""
import sys, subprocess, os, json
B = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12'
M = B + '/metrics/blockmetrics'; S = B + '/local/struct'
out = {}
for f in sys.argv[2:]:
    name = os.path.basename(f)
    base = name
    for suf in ('.routed.svg', '.static.svg', '.svg'):
        if base.endswith(suf): base = base[:-len(suf)]; break
    args = [M, '--detail', '--no-header']
    st = os.path.join(S, base + '.struct.json')
    if os.path.exists(st): args += ['--struct', st]
    r = subprocess.run(args + [f], capture_output=True, text=True)
    L = r.stderr.splitlines()
    ks = [i for i, l in enumerate(L) if l.strip() == 'labels:']
    labs = []
    if ks:
        k = ks[0]; h = L[k + 1]
        if 'detach' not in h or ' text' not in h:
            out[name] = labs; continue
        i = h.index('detach'); t = h.index(' text'); x = h.index(' x ', t + 5)
        for l in L[k + 2:]:
            if not l.startswith('  ') or l.strip() == '': break
            try: d = float(l[i - 2:i + 8].split()[0])
            except Exception: continue
            labs.append([l[:t].strip(), l[t:x].strip(), d])
    out[name] = labs
json.dump(out, open(sys.argv[1], 'w'))
print(len(out), 'svgs', sum(len(v) for v in out.values()), 'labels')
