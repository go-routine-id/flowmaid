#!/usr/bin/env python3
"""SPEC 2.7 F2 jog room, by its letter, on a drawn SVG.
k = sum over the title row of max(0, border(m) - cap(m)); growth = ROW_CLEAR + (k-1)*TRACK_PITCH.
Compared with the number of border links that really cross the title side at m
(TD: links into m from a node drawn above the frame, or out of m to one above)."""
import json, re, subprocess, sys
PORT_SPACING, CHIP_CLEAR, ROW_CLEAR, TRACK_PITCH = 14.0, 6.5, 12.5, 12.5
B = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12'
svg, struct = sys.argv[1], sys.argv[2]
st = json.load(open(struct))
raw = open(svg).read()
# node boxes from blockmetrics' model is not exported; take the frames from it and the nodes from the SVG <g> order
m = json.loads(subprocess.run([B + '/metrics/blockmetrics', '--json', '--struct', struct, svg], capture_output=True, text=True).stdout)[0]
frames = {f['id']: f for f in m['detail']['frames']}
# chips: rect fill-opacity 0.9 following each frame
chips = [(float(a), float(b), float(c), float(d)) for a, b, c, d in re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"[^>]*fill-opacity="0.9"', raw)]
gs = [mm.start() for mm in re.finditer(r'\n<g', raw)]
nb = {}
for i, g0 in enumerate(gs):
    seg = raw[g0:g0 + 600]
    r = re.search(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"', seg)
    if not r:
        print('node', i, 'is no rect: skipped', st['nodes'][i]['id']); continue
    x, y, w, h = map(float, r.groups())
    nb[st['nodes'][i]['id']] = {'x': x, 'y': y, 'w': w, 'h': h}
subs = st['subgraphs']; ns = st['nodes']; es = st['edges']
def inside(v, si):
    s = ns[v]['sub']
    while s is not None:
        if s == si: return True
        s = subs[s]['parent']
    return False
tot = 0
for si, s in enumerate(subs):
    f = frames.get(s['id'])
    if not f: continue
    chip = min(chips, key=lambda c: abs(c[0] - (f['x'] + 6)) + abs(c[1] - (f['y'] + 5)))
    c0, c1 = chip[0] - CHIP_CLEAR, chip[0] + chip[2] + CHIP_CLEAR
    mem = [v for v in range(len(ns)) if inside(v, si)]
    top = min(nb[ns[v]['id']]['y'] for v in mem)
    # title row (TD): members whose top is the block's first row
    row = [v for v in mem if nb[ns[v]['id']]['y'] - top < 30]
    k = 0; real = 0; out = []
    for v in row:
        b = nb[ns[v]['id']]
        border = [e for e in es if ('n' in e['from'] and 'n' in e['to']) and ((e['from']['n'] == v) != (e['to']['n'] == v)) and (inside(e['from']['n'], si) != inside(e['to']['n'], si))]
        border += [e for e in es if ('s' in e['from'] or 's' in e['to']) and (e['from'].get('n') == v or e['to'].get('n') == v)]
        # ports that fit on the title-facing side outside the chip's widened interval
        x0, x1 = b['x'], b['x'] + b['w']
        free = max(0.0, min(x1, c0) - x0) + max(0.0, x1 - max(x0, c1))
        if c1 <= x0 or c0 >= x1: free = b['w']
        cap = int(free // PORT_SPACING)
        # links that really cross the title side at m: other end drawn above the frame
        def other(e):
            o = e['to'] if e['from'].get('n') == v else e['from']
            if 'n' in o: return nb[ns[o['n']]['id']]['y']
            return frames[subs[o['s']]['id']]['y']
        cross = sum(1 for e in border if other(e) < f['y'])
        disp = max(0, len(border) - cap)
        real_disp = max(0, cross - cap)
        k += disp; real += real_disp
        out.append(f"{ns[v]['id']}: border {len(border)} (cross title side {cross}) cap {cap} -> displaced {disp} (needed {real_disp})")
    grow = ROW_CLEAR + (k - 1) * TRACK_PITCH if k > 0 else 0.0
    need = ROW_CLEAR + (real - 1) * TRACK_PITCH if real > 0 else 0.0
    tot += grow - need
    print(f"{s['id']}: frame h {f['h']:.0f} compact {f['compact']:.2f}; k={k} -> grows {grow:.1f} px (really needed {need:.1f}); compact after {f['compact']*(f['h']+grow)/f['h']:.2f} (needed-only {f['compact']*(f['h']+need)/f['h']:.2f})")
    for o in out: print('    ', o)
