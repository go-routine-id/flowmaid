"""BFS over (folded, zoom, ox, oy) from the state right after 'g'.
Phase 1 (zoom still 1.0): at most MAXPRE presses, dirty allowed.
Phase 2 (after the first zoom key): every state clean and non-empty.
Report reachable folded phase-2 states with rich content."""
import sys
sys.path.insert(0, '.')
import sim
from collections import deque

def ev(cols, rows, v, folded):
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a)
    d, w = sim.defects(b, t, a)
    clipped = [x for x in t if x['drawn'] != x['text']]
    full = sorted(x['name'] for x in b if x['rect'] == x['full'])
    return (not d and not clipped), len(b), full, sorted(x['text'] for x in t)

def search(cols, rows, MAXPRE=3, MAXPOST=16):
    v0 = sim.View(); v0.page_rows = rows - 9
    def key(f, v): return (f, round(v.zoom, 6), round(v.ox, 4), round(v.oy, 4))
    start = (False, v0)
    seen = {key(False, v0): ''}
    q = deque([(False, v0, '', False)])
    rich = []
    while q:
        f, v, path, zoomed = q.popleft()
        npre = len(path) if not zoomed else None
        if not zoomed and len(path) >= MAXPRE + 1:
            continue
        if zoomed:
            zi = next(i for i, c in enumerate(path) if c in '+-')
            if len(path) - zi > MAXPOST: continue
        for k in 'Bhjkl-+DU':
            nf = (not f) if k == 'B' else f
            n = sim.View(); n.ox, n.oy, n.zoom, n.page_rows = v.ox, v.oy, v.zoom, v.page_rows
            if k != 'B': n.key(k)
            nz = zoomed or k in '+-'
            if not zoomed and k in '+-' and False: pass
            kk = key(nf, n)
            if kk in seen: continue
            clean, nbox, full, toks = ev(cols, rows, n, nf)
            if nz and (not clean or nbox == 0):
                continue
            if not nz and k in 'hjklDU' and len(path) >= MAXPRE:
                continue
            seen[kk] = path + k
            q.append((nf, n, path + k, nz))
            if nz and nf:
                rich.append((len(full), len(toks), path + k, full, toks, round(n.zoom, 4)))
    return rich

if __name__ == '__main__':
    cols, rows = int(sys.argv[1]), int(sys.argv[2])
    r = search(cols, rows, int(sys.argv[3]) if len(sys.argv) > 3 else 3)
    r.sort(key=lambda x: (-x[0], -x[1], len(x[2])))
    seen_paths = set()
    for x in r[:25]:
        print(x)
