#!/usr/bin/env python3
"""B12.0: write tests/fixtures/blocks/ from the workspace sources (run once)."""
import os, re, sys, shutil
B = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12'
OUT = '/home/user/flowmaid/tests/fixtures/blocks'
os.makedirs(OUT, exist_ok=True)

def put(name, text):
    if not text.endswith('\n'):
        text += '\n'
    open(os.path.join(OUT, name + '.mmd'), 'w').write(text)

def copy(src, name=None):
    text = open(os.path.join(B, src)).read()
    put(name or os.path.basename(src)[:-4], text)

pub = open(os.path.join(B, 'revise/in/blocks-overview.mmd')).read()
put('blocks-overview', pub)
lines = pub.rstrip('\n').split('\n')
# header comment (4 lines), 'flowchart TD', actors (2), blank, six subgraph blocks, trailing edges
hdr = [l for l in lines if l.startswith('%%')]
first_edge = next(i for i, l in enumerate(lines) if l.strip().startswith('SHOP -->|browse|'))
edges = lines[first_edge:]
assert len(edges) == 22, len(edges)
body = lines[len(hdr):first_edge]
def note(what):
    return ['%% Block layout fixture: the storefront platform of blocks-overview.mmd',
            '%% with ' + what + '; the graph is the same.']
put('blocks-overview-edges-reversed', '\n'.join(note('its trailing edge statements in reverse order') + body + edges[::-1]))
put('blocks-overview-edges-rotated', '\n'.join(note('its trailing edge statements rotated by seven') + body + edges[7:] + edges[:7]))
# actors reversed
b2 = list(body)
i_shop = next(i for i, l in enumerate(b2) if l.strip().startswith('SHOP(['))
i_ops = next(i for i, l in enumerate(b2) if l.strip().startswith('OPS(['))
b2[i_shop], b2[i_ops] = b2[i_ops], b2[i_shop]
put('blocks-overview-actors-reversed', '\n'.join(note('its two actors declared in the other order') + b2 + edges))
# subgraph blocks reversed: split body into top-level blocks
pre, blocks, cur, depth = [], [], None, 0
for l in body:
    s = l.strip()
    if s.startswith('subgraph '):
        if depth == 0:
            cur = []
        depth += 1
    if cur is not None:
        cur.append(l)
    elif depth == 0 and not blocks and s:
        pre.append(l)
    if s == 'end':
        depth -= 1
        if depth == 0:
            blocks.append(cur); cur = None
assert len(blocks) == 6, len(blocks)
rev = pre + ['']
for blk in blocks[::-1]:
    rev += blk + ['']
put('blocks-overview-subgraphs-reversed', '\n'.join(note('its top-level subgraph blocks declared in reverse order') + rev + edges))
for d in ['BT', 'LR', 'RL']:
    t = '\n'.join(note('the flow direction ' + d) + body + edges).replace('flowchart TD', 'flowchart ' + d, 1)
    put('blocks-overview-' + d.lower(), t)

for n in ['hub5', 'cycle2', 'ratio2', 'ratio3', 'ratio4', 'cycle2-tie', 'cycle2-tie-swapped', 'interleave', 'nested',
          'self-member', 'sub-nested-self', 'b9-inside', 'b9-inside2', 'state-note', 'note-edge-row']:
    copy('critic/in/%s.mmd' % n)
put('lr', '''flowchart LR
    SHOP([shopper])
    subgraph EDGE[Edge and shared services with a long title]
        CDN[cdn]
        PAGER[pager]
    end
    subgraph WEB[Storefront]
        gate[gateway] --> cart[cart] --> db[(orders)]
    end
    subgraph ORD[Orders]
        queue[queue] --> ship[shipping]
    end
    SHOP -->|browse| CDN
    CDN -->|miss| gate
    CDN -->|hook| queue
    cart -->|page| PAGER
    ship -.->|page| PAGER
    db -->|order| ship
''')
copy('d-hybrid/in/swimlane.mmd'); copy('d-hybrid/in/nested-cases.mmd')
for f in sorted(os.listdir(os.path.join(B, 'u-mermaid/probes'))):
    if f.endswith('.mmd'):
        copy('u-mermaid/probes/' + f)
te = open(os.path.join(B, 'spec/evidence/title-entry.mmd')).read()
te = te.replace('A long block title that covers the first member · region one<br/>second line of the title',
                'A long block title that covers the first member and more<br/>second line of the title')
assert 'region one' not in te
put('title-entry', te)
for n in ['reqresp', 'reqresp22', 't9-frames-8px', 't9b-frames-8px-lr', 't1-nest-own', 't2-self-nested',
          't2b-self-nested-order', 'iso-lane', 'iso-state']:
    copy('check-fix-a/in/%s.mmd' % n)
copy('check-verify-fix-a/in/s3.mmd', 't9c-frames-8px-two-way')
for n in ['l5loop', 'swim-cycle', 'swim-nested', 'ratio2x4', 'ratio4mask', 'selfsub-nested', 'nested-self-only',
          'own-centre', 'title-hidden']:
    copy('check-fix-b/in/%s.mmd' % n)
copy('check-cold-read/apath-nested.mmd', 'nested-chain')
copy('check-revise-2/in/swimlane-nested.mmd')
il = open(os.path.join(B, 'check-fix-a/in/iso-lane.mmd')).read()
put('iso-lane-lr', il.replace('flowchart TD', 'flowchart LR', 1))
put('iso-nest', '''%% Not in block mode: an isolated two-level nest. No link joins a
%% subgraph to anything outside it.
flowchart TD
    x --> y
    subgraph OUT[Platform]
        subgraph IN[Workers]
            i1[w1] --> i2[w2] --> i3[w3]
            i3 -->|retry| i1
        end
    end
''')
put('empty-target', '''%% Not in block mode: the only edge to a subgraph ends on an empty
%% one, which is no unit and has no frame; its sub-edge is skipped.
flowchart TD
    x --> y
    subgraph S[Service]
        a --> b
    end
    subgraph EMPTY[Nothing here]
    end
    x --> EMPTY
''')
put('wide-stroke', '''%% Block layout fixture with a default stroke width: the orthogonal
%% edge stage does not run on such a graph, so the builders round
%% their own corners.
flowchart TD
    in[request]
    subgraph A[Front]
        a1[form] --> a2[page]
    end
    subgraph B[Middle]
        b1[check] --> b2[store]
    end
    subgraph C[Back]
        c1[report] --> c2[archive]
    end
    in --> a1
    a2 -->|submit| b1
    b2 -->|log| c1
    a1 -->|skip| c2
    c2 -.->|done| a2
    linkStyle default stroke-width:3px
''')
print(len(os.listdir(OUT)), 'fixtures')
