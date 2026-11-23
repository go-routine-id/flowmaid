"""Search key sequences after 'g' (unfolded, zoom 1, origin):
pre-zoom keys (fold + pans, dirty allowed but counted), then zoom keys and
pans that must ALL be clean and non-empty."""
import sys, itertools
sys.path.insert(0, '.')
import sim
from collections import deque

def ev(cols, rows, v, folded):
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a)
    d, w = sim.defects(b, t, a)
    clipped = [x for x in t if x['drawn'] != x['text']]
    full = [x['name'] for x in b if x['rect'] == x['full']]
    return dict(clean=not d and not clipped, nbox=len(b), full=full,
                toks=[x['text'] for x in t], hcut=[x['name'] for x in b if x['rect'][0] != x['full'][0] or x['rect'][2] != x['full'][2]])

def run(cols, rows, keys):
    v = sim.View(); v.page_rows = rows - 9
    folded = False
    out = []
    for k in keys:
        if k == 'B':
            folded = not folded
        else:
            v.key(k)
        out.append((k, ev(cols, rows, v, folded), v.zoom, v.ox, v.oy, folded))
    return out

if __name__ == '__main__':
    cols, rows = int(sys.argv[1]), int(sys.argv[2])
    best = []
    PRE = ['B', 'DB', 'BD', 'jB', 'Bj', 'jjB', 'BDl', 'BlD', 'BDll', 'DlB', 'BUl', 'BU']
    # pre-zoom options then zoom presses then BFS of clean nonempty states
    for pre in PRE:
        for zk in ['-', '--', '---', '+', '-l', '-ll']:
            seq = pre + zk
            r = run(cols, rows, seq)
            # states after first zoom must be clean
            zi = next(i for i, x in enumerate(seq) if x in '+-')
            if not all(x[1]['clean'] and x[1]['nbox'] > 0 for x in r[zi:]):
                continue
            dirty_pre = sum(1 for x in r[:zi] if not x[1]['clean'])
            # BFS over clean nonempty states
            v = sim.View(); v.page_rows = rows - 9
            for k in seq:
                if k != 'B': v.key(k)
            start = (round(v.ox, 4), round(v.oy, 4))
            seen = {start: ''}
            q = deque([(v, '')])
            goods = []
            while q:
                cv, path = q.popleft()
                if len(path) >= 14: continue
                for k in 'hjkl':
                    n = sim.View(); n.ox, n.oy, n.zoom, n.page_rows = cv.ox, cv.oy, cv.zoom, cv.page_rows
                    n.key(k)
                    kk = (round(n.ox, 4), round(n.oy, 4))
                    if kk in seen: continue
                    e = ev(cols, rows, n, True)
                    if not e['clean'] or e['nbox'] == 0: continue
                    seen[kk] = path + k
                    q.append((n, path + k))
                    goods.append((len(e['full']), len(e['toks']), -len(e['hcut']), path + k, e['full']))
            goods.sort(key=lambda g: (-g[0], -g[1], -g[2], len(g[3])))
            print(cols, rows, 'pre=%r zoom=%r dirty_pre=%d reach=%d' % (pre, zk, dirty_pre, len(seen)),
                  'zoomstate full=%s toks=%s' % (r[-1][1]['full'], r[-1][1]['toks']), 'best:', goods[:2])
