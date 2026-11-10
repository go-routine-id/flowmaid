#!/usr/bin/env python3
"""Measure block structure in mermaid SVG geometry (from extract.js).

usage: analyze.py <meas.json> <key> <axis: y for TD/TB, x for LR> [--full]
Prints markdown: blocks, bands (layers of top-level items), gaps, edge runs per gap,
edges through foreign blocks / nodes, label placement, title strikes, totals.
All numbers are px in the SVG's own user space (origin = top-left of the viewBox).
"""
import json, math, re, sys

EPS = 1.5


def rect(o):
    return (o['x'], o['y'], o['x'] + o['w'], o['y'] + o['h'])


def contains(outer, inner, tol=1.0):
    return outer[0] - tol <= inner[0] and outer[1] - tol <= inner[1] and outer[2] + tol >= inner[2] and outer[3] + tol >= inner[3]


def inside(r, p, m=0.0):
    return r[0] + m < p['x'] < r[2] - m and r[1] + m < p['y'] < r[3] - m


def dist_rect(r, p):
    dx = max(r[0] - p['x'], 0, p['x'] - r[2]); dy = max(r[1] - p['y'], 0, p['y'] - r[3])
    return math.hypot(dx, dy)


def dist_border(r, p):
    if inside(r, p):
        return min(p['x'] - r[0], r[2] - p['x'], p['y'] - r[1], r[3] - p['y'])
    return dist_rect(r, p)


def boxes_overlap(a, b, m=0.0):
    return a[0] + m < b[2] and b[0] + m < a[2] and a[1] + m < b[3] and b[1] + m < a[3]


def seg_len(a, b):
    return math.hypot(b['x'] - a['x'], b['y'] - a['y'])


def seg_cross(p1, p2, p3, p4):
    def o(a, b, c):
        return (b['x'] - a['x']) * (c['y'] - a['y']) - (b['y'] - a['y']) * (c['x'] - a['x'])
    d1, d2, d3, d4 = o(p3, p4, p1), o(p3, p4, p2), o(p1, p2, p3), o(p1, p2, p4)
    return ((d1 > 0) != (d2 > 0)) and ((d3 > 0) != (d4 > 0)) and abs(d1) > 1e-9 and abs(d2) > 1e-9 and abs(d3) > 1e-9 and abs(d4) > 1e-9


class Model:
    def __init__(self, v, axis):
        self.v = v; self.axis = axis; self.cross = 'x' if axis == 'y' else 'y'
        self.clusters = [dict(c, id=re.sub(r'^state-', '', re.sub(r'-\d+$', '', c['id'])) if c['id'].startswith('state-') else c['id']) for c in v['clusters']]
        self.nodes = [n for n in v['nodes'] if n['w'] > 0.5 and n['h'] > 0.5]
        for c in self.clusters:
            c['r'] = rect(c)
        for n in self.nodes:
            n['r'] = rect(n); n['c'] = {'x': n['x'] + n['w'] / 2, 'y': n['y'] + n['h'] / 2}
        # hierarchy by containment
        for c in self.clusters:
            parents = [p for p in self.clusters if p is not c and contains(p['r'], c['r']) and (p['w'] * p['h'] > c['w'] * c['h'])]
            c['parent'] = min(parents, key=lambda p: p['w'] * p['h'])['id'] if parents else None
        self.cby = {c['id']: c for c in self.clusters}
        for c in self.clusters:
            d = 0; p = c['parent']
            while p:
                d += 1; p = self.cby[p]['parent']
            c['depth'] = d
        for n in self.nodes:
            ins = [c for c in self.clusters if inside(c['r'], n['c'])]
            n['cluster'] = min(ins, key=lambda c: c['w'] * c['h'])['id'] if ins else None
        self.nby = {}
        for n in self.nodes:
            self.nby.setdefault(n['id'], n)
        self.edges = v['edges']; self.labels = v['labels']
        self.resolve_edges()
        self.make_bands()

    # ---- helpers
    def lo(self, r):
        return r[1] if self.axis == 'y' else r[0]

    def hi(self, r):
        return r[3] if self.axis == 'y' else r[2]

    def clo(self, r):
        return r[0] if self.axis == 'y' else r[1]

    def chi(self, r):
        return r[2] if self.axis == 'y' else r[3]

    def top_of(self, kind, ident):
        """top-level item id ('c:ID' or 'n:ID') holding a node or cluster"""
        if kind == 'n':
            c = self.nby[ident]['cluster']
            if c is None:
                return 'n:' + ident
        else:
            c = ident
        while self.cby[c]['parent']:
            c = self.cby[c]['parent']
        return 'c:' + c

    def ancestors(self, kind, ident):
        out = set()
        c = self.nby[ident]['cluster'] if kind == 'n' else ident
        while c:
            out.add(c); c = self.cby[c]['parent']
        return out

    def resolve_end(self, p):
        best = None
        for n in self.nodes:
            d = dist_rect(n['r'], p)
            if best is None or d < best[0]:
                best = (d, 'n', n['id'])
        bc = None
        for c in self.clusters:
            d = dist_border(c['r'], p)
            if bc is None or d < bc[0]:
                bc = (d, 'c', c['id'])
        if best and best[0] <= 8:
            return best[1], best[2]
        if bc and bc[0] <= 8:
            return bc[1], bc[2]
        cands = [x for x in (best, bc) if x]
        m = min(cands, key=lambda x: x[0])
        return m[1], m[2]

    def resolve_edges(self):
        ids = {n['id']: 'n' for n in self.nodes}
        for c in self.clusters:
            ids.setdefault(c['id'], 'c')
        for e in self.edges:
            pts = e['points'] or e['samp']
            e['pl'] = pts
            src = dst = None
            m = re.match(r'^L_(.+)_(\d+)$', e['id'])
            if m:
                parts = m.group(1).split('_')
                for i in range(1, len(parts)):
                    a, b = '_'.join(parts[:i]), '_'.join(parts[i:])
                    if a in ids and b in ids:
                        src, dst = (ids[a], a), (ids[b], b); break
            if src is None:
                src = self.resolve_end(e['samp'][0]); dst = self.resolve_end(e['samp'][-1])
            e['src'], e['dst'] = src, dst
            e['stop'], e['dtop'] = self.top_of(*src), self.top_of(*dst)
            e['kind'] = 'intra' if e['stop'] == e['dtop'] else 'inter'
            e['toblock'] = (src[0] == 'c') or (dst[0] == 'c')

    def make_bands(self):
        items = []
        for c in self.clusters:
            if c['parent'] is None:
                items.append({'id': 'c:' + c['id'], 'r': c['r'], 'label': '[' + c['id'] + ']'})
        for n in self.nodes:
            if n['cluster'] is None:
                items.append({'id': 'n:' + n['id'], 'r': n['r'], 'label': n['id']})
        items.sort(key=lambda it: (self.lo(it['r']), self.clo(it['r'])))
        bands = []
        for it in items:
            if bands and self.lo(it['r']) < bands[-1]['hi'] - 2:
                bands[-1]['items'].append(it); bands[-1]['hi'] = max(bands[-1]['hi'], self.hi(it['r']))
            else:
                bands.append({'lo': self.lo(it['r']), 'hi': self.hi(it['r']), 'items': [it]})
        for b in bands:
            b['items'].sort(key=lambda it: self.clo(it['r']))
        self.items = items; self.bands = bands
        self.band_of = {}
        for i, b in enumerate(bands):
            for it in b['items']:
                self.band_of[it['id']] = i

    def locate(self, c, lo_c, hi_c):
        """where is a cross-flow run at flow coordinate c spanning cross range [lo_c,hi_c]"""
        if c < self.bands[0]['lo']:
            return 'before band 0'
        for i, b in enumerate(self.bands):
            if b['lo'] - 0.5 <= c <= b['hi'] + 0.5:
                hits = []
                for it in b['items']:
                    r = it['r']
                    if self.lo(r) + 0.5 < c < self.hi(r) - 0.5 and self.clo(r) < hi_c and lo_c < self.chi(r):
                        hits.append(it['label'])
                if hits:
                    return 'band %d inside %s' % (i, '+'.join(hits))
                return 'band %d beside blocks' % i
            if i + 1 < len(self.bands) and b['hi'] < c < self.bands[i + 1]['lo']:
                return 'gap %d-%d' % (i, i + 1)
        return 'after last band'


def fmt(x):
    return ('%.0f' % x)


def analyze(name, v, axis, full=False):
    M = Model(v, axis)
    out = []
    A = 'y' if axis == 'y' else 'x'
    flow_extent = v['height'] if axis == 'y' else v['width']
    out.append('#### %s  (canvas %s x %s, flow axis %s)' % (name, fmt(v['width']), fmt(v['height']), A))
    out.append('')
    # --- blocks
    out.append('Blocks (cluster rectangles):')
    out.append('')
    out.append('| block | depth | parent | x | y | w | h | share of flow extent | member nodes | inner layers (nodes per layer, in flow order) |')
    out.append('|---|---|---|---|---|---|---|---|---|---|')
    for c in sorted(M.clusters, key=lambda c: (c['depth'], M.lo(c['r']), M.clo(c['r']))):
        mem = [n for n in M.nodes if n['cluster'] == c['id']]
        allmem = [n for n in M.nodes if c['id'] in M.ancestors('n', n['id'])]
        rows = []
        for n in sorted(allmem, key=lambda n: M.lo(n['r'])):
            if rows and M.lo(n['r']) < rows[-1][0] - 2:
                rows[-1][1].append(n['id']); rows[-1][0] = max(rows[-1][0], M.hi(n['r']))
            else:
                rows.append([M.hi(n['r']), [n['id']]])
        share = (M.hi(c['r']) - M.lo(c['r'])) / flow_extent * 100
        out.append('| %s | %d | %s | %s | %s | %s | %s | %.0f%% | %d (%d incl. nested) | %s |' % (
            c['id'], c['depth'], c['parent'] or '-', fmt(c['x']), fmt(c['y']), fmt(c['w']), fmt(c['h']), share, len(mem), len(allmem),
            ' / '.join(str(len(r[1])) for r in rows)))
    out.append('')
    # --- bands
    out.append('Bands of top-level items along the flow (a band = items whose flow intervals overlap; `[X]` = block):')
    out.append('')
    out.append('| band | from | to | size | items in cross-axis order | gap to next band |')
    out.append('|---|---|---|---|---|---|')
    for i, b in enumerate(M.bands):
        gap = (M.bands[i + 1]['lo'] - b['hi']) if i + 1 < len(M.bands) else None
        out.append('| %d | %s | %s | %s | %s | %s |' % (i, fmt(b['lo']), fmt(b['hi']), fmt(b['hi'] - b['lo']), ' , '.join(it['label'] for it in b['items']), fmt(gap) if gap is not None else '-'))
    out.append('')
    nblocks_top = [c for c in M.clusters if c['parent'] is None]
    stag = []
    for i, b in enumerate(M.bands):
        core_lo = max(M.lo(it['r']) for it in b['items']); core_hi = min(M.hi(it['r']) for it in b['items'])
        b['clean'] = core_lo < core_hi
        if not b['clean']:
            stag.append(i)
    out.append('Top-level blocks: %d; bands: %d; staggered bands (items chained by partial overlap, no flow coordinate common to all of them - i.e. not one layer): %s.' % (
        len(nblocks_top), len(M.bands), ', '.join('band %d' % i for i in stag) if stag else 'none'))
    out.append('')
    # side-by-side gaps inside bands
    rowsb = []
    for i, b in enumerate(M.bands):
        its = b['items']
        for a, c in zip(its, its[1:]):
            g = M.clo(c['r']) - M.chi(a['r'])
            rowsb.append('band %d: %s -> %s = %s' % (i, a['label'], c['label'], fmt(g)))
    if rowsb:
        out.append('Cross-axis gaps between neighbours in the same band: ' + '; '.join(rowsb) + '.')
        out.append('')
    # --- edges
    inter = [e for e in M.edges if e['kind'] == 'inter']
    intra = [e for e in M.edges if e['kind'] == 'intra']
    gap_use = {}
    edge_rows = []
    through_rows = []
    node_hits = []
    for e in M.edges:
        pl = e['pl']
        runs = []
        for a, b in zip(pl, pl[1:]):
            L = seg_len(a, b)
            if L < 8:
                continue
            dflow = abs(b[A] - a[A]); dcross = abs(b[M.cross] - a[M.cross])
            if dcross > 3 * dflow:  # cross-flow run
                c = (a[A] + b[A]) / 2
                where = M.locate(c, min(a[M.cross], b[M.cross]), max(a[M.cross], b[M.cross]))
                runs.append((where, L, c))
        e['runs'] = runs
        if e['kind'] == 'inter':
            for where, L, c in runs:
                g = gap_use.setdefault(where, {'edges': set(), 'len': 0.0, 'cs': []})
                g['edges'].add(e['id']); g['len'] += L; g['cs'].append(c)
        # through foreign blocks
        anc = M.ancestors(*e['src']) | M.ancestors(*e['dst'])
        for c in M.clusters:
            if c['id'] in anc:
                continue
            cnt = sum(1 for p in e['samp'] if inside(c['r'], p, 2.0))
            if cnt >= 2:
                through_rows.append((e['id'], c['id'], cnt * e['len'] / max(1, len(e['samp']) - 1)))
        ends = {e['src'][1], e['dst'][1]}
        for n in M.nodes:
            if n['id'] in ends:
                continue
            cnt = sum(1 for p in e['samp'] if inside(n['r'], p, 1.5))
            if cnt >= 2:
                node_hits.append((e['id'], n['id'], cnt * e['len'] / max(1, len(e['samp']) - 1)))
    sides = {}
    for e in inter:
        for role, top in (('leaves', e['stop']), ('enters', e['dtop'])):
            if not top.startswith('c:'):
                continue
            r = M.cby[top[2:]]['r']
            seq = e['samp'] if role == 'leaves' else e['samp'][::-1]
            # walk from the end that lies in/on the block until the first sample clearly outside
            prev = seq[0]; side = None
            for p in seq[1:]:
                if dist_rect(r, p) > 1.0:
                    q = prev
                    cand = {'top': abs(q['y'] - r[1]), 'bottom': abs(q['y'] - r[3]), 'left': abs(q['x'] - r[0]), 'right': abs(q['x'] - r[2])}
                    side = min(cand, key=cand.get); break
                prev = p
            if side:
                k = (role, side)
                sides[k] = sides.get(k, 0) + 1
                e.setdefault('sides', {})[role] = side
    if sides:
        out.append('Sides of block frames crossed by inter-block edges: ' + ', '.join('%s via %s: %d' % (k[0], k[1], n) for k, n in sorted(sides.items())) + '.')
        out.append('')
    out.append('Edges: %d total, %d with both ends in the same top-level block (intra) and %d between different top-level items (inter); %d have a block (not a node) as an end.' % (
        len(M.edges), len(intra), len(inter), sum(1 for e in M.edges if e['toblock'])))
    out.append('')
    out.append('Where the cross-flow runs (the "bus" segments, >= 8 px) of inter-block edges lie:')
    out.append('')
    out.append('| place | edges with a run here | total run length | distinct lanes (flow coordinates) |')
    out.append('|---|---|---|---|')

    def keyf(k):
        m = re.search(r'(\d+)', k)
        return (int(m.group(1)) if m else -1, k)
    for k in sorted(gap_use, key=keyf):
        g = gap_use[k]
        lanes = sorted(set(round(c) for c in g['cs']))
        out.append('| %s | %d | %s | %d%s |' % (k, len(g['edges']), fmt(g['len']), len(lanes), (' (' + ', '.join(str(l) for l in lanes[:14]) + (' ...' if len(lanes) > 14 else '') + ')')))
    out.append('')
    if full:
        out.append('| inter-block edge | from (block) | to (block) | bands | cross-flow runs |')
        out.append('|---|---|---|---|---|')
        for e in inter:
            out.append('| %s | %s (%s) | %s (%s) | %d -> %d | %s |' % (
                e['id'], e['src'][1], e['stop'][2:], e['dst'][1], e['dtop'][2:], M.band_of.get(e['stop'], -1), M.band_of.get(e['dtop'], -1),
                '; '.join('%s %s' % (w, fmt(L)) for w, L, c in e['runs']) or 'straight'))
        out.append('')
    if through_rows:
        out.append('Edges drawn through a block they do not belong to (%d cases): ' % len(through_rows) + '; '.join('%s through [%s] for %s px' % (a, b, fmt(c)) for a, b, c in through_rows) + '.')
    else:
        out.append('Edges drawn through a block they do not belong to: none.')
    out.append('')
    if node_hits:
        out.append('Edges drawn over a node that is not their end (%d cases): ' % len(node_hits) + '; '.join('%s over %s for %s px' % (a, b, fmt(c)) for a, b, c in node_hits) + '.')
    else:
        out.append('Edges drawn over a node that is not their end: none.')
    out.append('')
    # title strikes
    strikes = []
    for c in M.clusters:
        tb = c.get('titleBox')
        if not tb or tb['w'] < 2:
            continue
        r = rect(tb)
        for e in M.edges:
            cnt = sum(1 for p in e['samp'] if inside(r, p, 0.5))
            if cnt >= 1:
                strikes.append('%s over title of [%s]' % (e['id'], c['id']))
    out.append('Edges drawn over a block title: %s.' % ('; '.join(strikes) if strikes else 'none'))
    out.append('')
    # --- labels
    eby = {e['id']: e for e in M.edges}
    if M.labels:
        out.append('Edge labels:')
        out.append('')
        out.append('| label of edge | size | place | off its own line | path distance to nearer end | fraction along path | overlaps |')
        out.append('|---|---|---|---|---|---|---|')
        lrects = [(l, rect(l)) for l in M.labels]
        far = 0; off = 0; nover = 0
        for l, r in lrects:
            e = eby.get(l['id'])
            c = {'x': l['x'] + l['w'] / 2, 'y': l['y'] + l['h'] / 2}
            dline = frac = dend = None
            if e:
                best = None
                for i, p in enumerate(e['samp']):
                    d = math.hypot(p['x'] - c['x'], p['y'] - c['y'])
                    if best is None or d < best[0]:
                        best = (d, i)
                dline = best[0]; frac = best[1] / max(1, len(e['samp']) - 1)
                dend = min(frac, 1 - frac) * e['len']
            where = M.locate(c[A], l[M.cross], l[M.cross] + (l['w'] if M.cross == 'x' else l['h']))
            ov = []
            for l2, r2 in lrects:
                if l2 is not l and boxes_overlap(r, r2, 1.0):
                    ov.append('label ' + l2['id'])
            for n in M.nodes:
                if boxes_overlap(r, n['r'], 1.0):
                    ov.append('node ' + n['id'])
            for cl in M.clusters:
                cr = cl['r']
                if boxes_overlap(r, cr, 1.0) and not contains(cr, r, -1.0):
                    ov.append('frame of [' + cl['id'] + ']')
                tb = cl.get('titleBox')
                if tb and tb['w'] > 2 and boxes_overlap(r, rect(tb), 1.0):
                    ov.append('title of [' + cl['id'] + ']')
            for e2 in M.edges:
                if e2['id'] == l['id']:
                    continue
                if sum(1 for p in e2['samp'] if inside(r, p, 1.0)) >= 1:
                    ov.append('line ' + e2['id'])
            if ov:
                nover += 1
            if dline is not None and dline > 12:
                off += 1
            if dend is not None and dend > 300:
                far += 1
            out.append('| %s | %sx%s | %s | %s | %s | %s | %s |' % (l['id'], fmt(l['w']), fmt(l['h']), where, fmt(dline) if dline is not None else '?', fmt(dend) if dend is not None else '?', ('%.2f' % frac) if frac is not None else '?', ', '.join(ov) or '-'))
        out.append('')
        lrows = {}
        for l, r in lrects:
            cflow = (l['y'] + l['h'] / 2) if axis == 'y' else (l['x'] + l['w'] / 2)
            lrows.setdefault(round(cflow / 4) * 4, []).append(l['id'])
        big = sorted(((k, ids) for k, ids in lrows.items() if len(ids) >= 3), key=lambda t: t[0])
        if big:
            out.append('Label rows (>= 3 labels centred on the same flow coordinate): ' + '; '.join('%s=%d: %d labels' % (A, k, len(ids)) for k, ids in big) + '.')
            out.append('')
        out.append('Labels: %d; more than 12 px off their own line: %d; more than 300 px of path from the nearer end: %d; overlapping something: %d.' % (len(M.labels), off, far, nover))
        out.append('')
    # --- totals
    tot = sum(e['len'] for e in M.edges)
    bends = sum(max(0, len(e['points']) - 2) for e in M.edges if e['points'])
    # crossings on coarse polylines
    pls = []
    for e in M.edges:
        s = e['points'] if (e['points'] and 'elk' in name) else e['samp'][::2] + [e['samp'][-1]]
        xs = [p['x'] for p in s]; ys = [p['y'] for p in s]
        pls.append((e, s, (min(xs), min(ys), max(xs), max(ys))))
    cross = 0
    for i in range(len(pls)):
        for j in range(i + 1, len(pls)):
            if not boxes_overlap(pls[i][2], pls[j][2], -0.5):
                continue
            ei, ej = pls[i][0], pls[j][0]
            shared = {ei['src'][1], ei['dst'][1]} & {ej['src'][1], ej['dst'][1]}
            k = 0
            for a, b in zip(pls[i][1], pls[i][1][1:]):
                for c, d in zip(pls[j][1], pls[j][1][1:]):
                    if max(a['x'], b['x']) < min(c['x'], d['x']) or max(c['x'], d['x']) < min(a['x'], b['x']) or max(a['y'], b['y']) < min(c['y'], d['y']) or max(c['y'], d['y']) < min(a['y'], b['y']):
                        continue
                    if seg_cross(a, b, c, d):
                        k += 1
            cross += k
    out.append('Totals: canvas area %.2f Mpx2, edge length %s px, bends %d, edge crossings (drawn polylines) %d.' % (v['width'] * v['height'] / 1e6, fmt(tot), bends, cross))
    out.append('')
    return '\n'.join(out), M


if __name__ == '__main__':
    d = json.load(open(sys.argv[1]))
    txt, _ = analyze(sys.argv[2], d[sys.argv[2]], sys.argv[3], '--full' in sys.argv)
    print(txt)
