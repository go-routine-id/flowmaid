# refcols.py old-metrics-dir new-metrics-dir: the columns each table changed in, and whether any rule that reads a
# reference table at judge time (a dict bound) reads a changed column of that table
import json, sys, csv
O, N = sys.argv[1], sys.argv[2]
def tab(p):
    r = list(csv.reader(open(p), delimiter='\t')); h = r[0]; return h, {x[0]: dict(zip(h, x)) for x in r[1:]}
changed = {}
for t in ['baseline-038.tsv', 'ref-start.tsv', 'ref-mermaid.tsv', 'ref-038-routed.tsv', 'ref-start-routed.tsv']:
    ho, o = tab(f'{O}/{t}'); hn, n = tab(f'{N}/{t}')
    assert ho == hn and list(o) == list(n), t
    cols = {}
    for name in o:
        for c in ho:
            if o[name][c] != n[name][c]: cols[c] = cols.get(c, 0) + 1
    changed[t] = cols
    print(f'{t:22} rows {len(o):3}  changed cells {sum(cols.values()):4}  columns {cols}')
def refs(b):
    if isinstance(b, dict):
        if 'ref' in b: yield b['ref']
        for k in ('min', 'max'):
            for x in b.get(k, []): yield from refs(x)
for jf, tabs in [('thresholds.json', {'baseline': 'baseline-038.tsv', 'start': 'ref-start.tsv', 'mermaid': 'ref-mermaid.tsv'}),
                 ('thresholds-routed.json', {'baseline': 'ref-038-routed.tsv', 'start': 'ref-start-routed.tsv', 'mermaid': 'ref-mermaid.tsv'})]:
    hit = []
    for r in json.load(open(f'{N}/{jf}'))['rules']:
        for f in ('must', 'target'):
            for ref in refs(r.get(f)):
                if r['metric'] in changed[tabs[ref]]: hit.append((r['metric'], f, ref))
    print(f'{jf}: rules reading a changed reference column at judge time: {hit if hit else "none"}')
