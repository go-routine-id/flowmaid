#!/usr/bin/env python3
# Seeded generator of block diagrams: nested subgraphs, direct links between
# members on neighbouring rows, links to whole blocks, turned links.
import random, sys, os
out = sys.argv[1]; N = int(sys.argv[2])
for k in range(N):
    r = random.Random(1000 + k)
    d = r.choice(['TD', 'TD', 'TD', 'LR', 'BT', 'RL'])
    nb = r.randint(2, 5)
    lines = [f'flowchart {d}']
    nodes = []          # (id, block path)
    blocks = []         # ids
    def block(name, depth, parent):
        blocks.append(name)
        mem = []
        n = r.randint(2, 5)
        body = []
        for i in range(n):
            nid = f'{name.lower()}n{i}'
            mem.append(nid); nodes.append((nid, name))
            body.append(f'{nid}[{name} {i}]')
        inner = []
        if depth < 2 and r.random() < 0.45:
            inner = block(name + 'x', depth + 1, name)
        return [(name, body, inner, mem)]
    tree = []
    for b in range(nb):
        tree += block('ABCDE'[b], 0, None)
    def emit(t, ind):
        name, body, inner, mem = t
        lines.append(' ' * ind + f'subgraph {name}[Block {name} title]')
        for l in body: lines.append(' ' * (ind + 2) + l)
        for i in inner: emit(i, ind + 2)
        lines.append(' ' * ind + 'end')
        # inner chain
    for t in tree: emit(t, 2)
    free = [f'f{i}' for i in range(r.randint(1, 4))]
    for f in free: lines.append(f'  {f}([free {f}])')
    ids = [n for n, _ in nodes] + free
    # chains inside blocks
    byb = {}
    for n, b in nodes: byb.setdefault(b, []).append(n)
    for b, ms in byb.items():
        for i in range(len(ms) - 1):
            if r.random() < 0.75: lines.append(f'  {ms[i]} --> {ms[i+1]}')
        if len(ms) > 2 and r.random() < 0.4: lines.append(f'  {ms[0]} --> {ms[-1]}')
    ne = r.randint(len(blocks) + 2, len(blocks) * 3 + 4)
    for _ in range(ne):
        a = r.choice(ids + blocks); b = r.choice(ids + blocks)
        if a == b: continue
        # no link from a block to its own member or nested block (own-box edges are B12.14's)
        def anc(x):
            if x in blocks:
                return [x[:i] for i in range(1, len(x) + 1)]
            for n, bl in nodes:
                if n == x: return [bl[:i] for i in range(1, len(bl) + 1)]
            return []
        if a in anc(b) or b in anc(a): continue
        lab = '|' + r.choice(['yes', 'no', 'sync', 'retry']) + '|' if r.random() < 0.2 else ''
        arrow = r.choice(['-->', '-->', '-->', '-.->', '==>'])
        lines.append(f'  {a} {arrow}{lab} {b}')
    open(os.path.join(out, f'g{k:03d}.mmd'), 'w').write('\n'.join(lines) + '\n')
