#!/usr/bin/env python3
"""detach.py <outfile> <svg...>
Runs blockmetrics --detail over the SVGs (struct dir of the local inputs when a
struct file exists) and writes, per SVG: name, chips detached > 3 px, > 5 px,
max detach, and the label lines over 3 px. Prints a summary line."""
import sys, subprocess, os, re
B = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12'
M = B + '/metrics/blockmetrics'
S = B + '/local/struct'
out = sys.argv[1]
svgs = sys.argv[2:]
rows = []
tot3 = tot5 = 0
for f in svgs:
    name = os.path.basename(f)
    for suf in ('.routed.svg', '.static.svg', '.svg'):
        if name.endswith(suf):
            name = name[: -len(suf)]
            break
    args = [M, '--detail', '--no-header']
    st = os.path.join(S, name + '.struct.json')
    if os.path.exists(st):
        args += ['--struct', st]
    r = subprocess.run(args + [f], capture_output=True, text=True)
    L = r.stderr.splitlines()
    ks = [i for i, l in enumerate(L) if l.strip() == 'labels:']
    n3 = n5 = 0
    mx = 0.0
    det = []
    if ks:
        k = ks[0]
        h = L[k + 1]
        if 'detach' in h:
            i = h.index('detach')
            t = h.index(' text')
            for l in L[k + 2:]:
                if not l.startswith('  ') or l.strip() == '':
                    break
                try:
                    d = float(l[i - 2:i + 8].split()[0])
                except Exception:
                    continue
                mx = max(mx, d)
                if d > 3:
                    n3 += 1
                    det.append('    %s | %s | %.1f' % (l[:t].strip(), l[t:t + 40].strip(), d))
                if d > 5:
                    n5 += 1
    tot3 += n3
    tot5 += n5
    rows.append((name, n3, n5, mx, det))
with open(out, 'w') as o:
    for name, n3, n5, mx, det in rows:
        o.write('%s\t%d\t%d\t%.1f\n' % (name, n3, n5, mx))
        for d in det:
            o.write(d + '\n')
print('detach: %d svgs, >3px %d, >5px %d' % (len(svgs), tot3, tot5))
