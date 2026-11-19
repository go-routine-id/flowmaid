# thrdiff.py old.json new.json: every rule whose must/target changed, with the direction (tighter / looser)
import json, sys
o = json.load(open(sys.argv[1]))['rules']; n = json.load(open(sys.argv[2]))['rules']
assert len(o) == len(n), (len(o), len(n))
tighter = looser = other = 0
for i, (a, b) in enumerate(zip(o, n)):
    ka = {k: v for k, v in a.items() if k not in ('must', 'target')}; kb = {k: v for k, v in b.items() if k not in ('must', 'target')}
    if ka != kb: print('RULE SHAPE CHANGED', i, a, b); other += 1; continue
    for f in ('must', 'target'):
        va, vb = a.get(f), b.get(f)
        if va == vb: continue
        if isinstance(va, (int, float)) and isinstance(vb, (int, float)):
            t = (vb < va) if a['op'] == '<=' else (vb > va) if a['op'] == '>=' else None
            kind = 'tighter' if t else 'LOOSER' if t is not None else 'CHANGED'
        else: kind = 'CHANGED (not numeric)'
        if kind == 'tighter': tighter += 1
        elif kind == 'LOOSER': looser += 1
        else: other += 1
        sc = a['scope'] if isinstance(a['scope'], str) else ', '.join(a['scope'])
        print(f"{kind:8} rule {i:3} {f:6} {a['metric']:10} {a['op']} {va} -> {vb}   scope: {sc}   ({a['why']})")
print(f'summary: {tighter} tighter, {looser} looser, {other} other, {len(o)} rules')
