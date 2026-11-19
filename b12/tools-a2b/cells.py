#!/usr/bin/env python3
"""cells.py <base-outdir> <cand-outdir>

Check 1: per-cell no-worse on the local inputs, judged by the LIVE judge (rejudge.sh output of both runs).
A cell is one rule line of the judge: (input, metric, rule). Two tables are compared:
  static  <outdir>/local/judge-A2-all.txt      every static rule due by A2 (judge --due A2 --all)
  routed  <outdir>/local/judge-routed-all.txt  every routed rule (judge-routed --all)
Verdict classes (a cell that is a MUST FAIL in either run):
  NEW      must-fail in the candidate, not in the baseline                    -> worse
  WORSE    must-fail in both, the value moved further to the wrong side        -> worse
  CHANGED  must-fail in both, a string value (bands) or NA changed            -> worse (no direction)
  MISSING  a baseline cell the candidate's judge did not print                 -> worse
  GONE     must-fail in the baseline only                                      (better)
  BETTER   must-fail in both, the value moved toward the bound                 (better)
Informational, not part of the verdict:
  target   target-missed lines that are new / gone / worse
  drift    passing cells whose value moved the wrong way (still within the must)
Exit status 0 when nothing is worse, 1 otherwise. --quiet prints only the counts and the verdict lines.
"""
import math
import re
import sys

LINE = re.compile(r'^(MUST FAIL|target missed|ok)\s+(\S+)\s+(\S+)\s+(.*?)  must (<=|>=|==) (.*?)\s+target (?:<=|>=|==) (.*?)\s+\[0\.38 .*?\]  ?(.*)$')


def num(s):
    s = s.strip()
    if s in ('NA', '-', ''):
        return None
    try:
        return float(s)
    except ValueError:
        return s


def load(path):
    cells, order, seen = {}, [], {}
    for line in open(path, errors='replace'):
        m = LINE.match(line.rstrip('\n'))
        if not m:
            continue
        verdict, name, metric, val, op, must, _target, why = m.groups()
        k0 = (name, metric, why.strip())
        seen[k0] = seen.get(k0, 0) + 1
        key = k0 + (seen[k0],)
        cells[key] = dict(verdict=verdict, value=num(val), op=op, must=must.strip(), why=why.strip())
        order.append(key)
    return cells, order


def fmt(v):
    if v is None:
        return 'NA'
    if isinstance(v, float):
        return ('%g' % v) if abs(v) < 1e6 else '%.0f' % v
    return v


def direction(op, a, b):
    """+1 worse, -1 better, 0 equal, None no direction"""
    if a == b:
        return 0
    if not isinstance(a, float) or not isinstance(b, float) or math.isnan(a) or math.isnan(b):
        return None
    if op == '<=':
        return 1 if b > a else -1
    if op == '>=':
        return 1 if b < a else -1
    return None


def compare(label, bpath, cpath, quiet):
    try:
        B, order = load(bpath)
        C, corder = load(cpath)
    except FileNotFoundError as e:
        print(f'[{label}] cannot read {e.filename}')
        return 1
    if not B:
        print(f'[{label}] no rule lines in {bpath} (was it made with --all?)')
        return 1
    cls = {k: [] for k in ['NEW', 'WORSE', 'CHANGED', 'MISSING', 'GONE', 'BETTER', 'target-new', 'target-gone', 'target-worse', 'drift']}
    for key in order + [k for k in corder if k not in B]:
        b, c = B.get(key), C.get(key)
        name, metric = key[0], key[1]
        tag = f'{name} {metric}'
        if c is None:
            if b['verdict'] == 'MUST FAIL':
                cls['MISSING'].append(f'{tag} {fmt(b["value"])} (must {b["op"]} {b["must"]})')
            continue
        if b is None:
            if c['verdict'] == 'MUST FAIL':
                cls['NEW'].append(f'{tag} {fmt(c["value"])} (must {c["op"]} {c["must"]}; no baseline cell)')
            continue
        bf, cf = b['verdict'] == 'MUST FAIL', c['verdict'] == 'MUST FAIL'
        d = direction(c['op'], b['value'], c['value'])
        moved = f'{tag} {fmt(b["value"])} -> {fmt(c["value"])} (must {c["op"]} {c["must"]})'
        if cf and not bf:
            cls['NEW'].append(f'{tag} {fmt(c["value"])} (must {c["op"]} {c["must"]}; baseline {fmt(b["value"])})')
        elif bf and not cf:
            cls['GONE'].append(moved)
        elif bf and cf:
            if d == 1:
                cls['WORSE'].append(moved)
            elif d == -1:
                cls['BETTER'].append(moved)
            elif d is None:
                cls['CHANGED'].append(moved)
        else:
            bt, ct = b['verdict'] == 'target missed', c['verdict'] == 'target missed'
            if ct and not bt:
                cls['target-new'].append(moved)
            elif bt and not ct:
                cls['target-gone'].append(moved)
            elif bt and ct and d == 1:
                cls['target-worse'].append(moved)
            elif d == 1:
                cls['drift'].append(moved)
    nb = sum(1 for v in B.values() if v['verdict'] == 'MUST FAIL')
    nc = sum(1 for v in C.values() if v['verdict'] == 'MUST FAIL')
    worse = sum(len(cls[k]) for k in ['NEW', 'WORSE', 'CHANGED', 'MISSING'])
    print(f'[{label}] {len(B)} / {len(C)} cells; must-fail {nb} -> {nc}; worse {worse} (new {len(cls["NEW"])}, worse inside failing {len(cls["WORSE"])}, '
          f'changed {len(cls["CHANGED"])}, missing {len(cls["MISSING"])}); gone {len(cls["GONE"])}, better inside failing {len(cls["BETTER"])}; '
          f'info: target new {len(cls["target-new"])} / gone {len(cls["target-gone"])} / worse {len(cls["target-worse"])}, drift inside passing {len(cls["drift"])}')
    for k in ['NEW', 'WORSE', 'CHANGED', 'MISSING']:
        for x in cls[k]:
            print(f'  {k:8} {x}')
    if not quiet:
        for k in ['GONE', 'BETTER', 'target-new', 'target-gone', 'target-worse', 'drift']:
            for x in cls[k]:
                print(f'  {k.lower():12} {x}')
    return 1 if worse else 0


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    quiet = '--quiet' in sys.argv
    if len(args) != 2:
        print(__doc__)
        sys.exit(2)
    b, c = args
    rc = 0
    rc |= compare('local static A2', f'{b}/local/judge-A2-all.txt', f'{c}/local/judge-A2-all.txt', quiet)
    rc |= compare('local routed', f'{b}/local/judge-routed-all.txt', f'{c}/local/judge-routed-all.txt', quiet)
    print('cells: no worse' if rc == 0 else 'cells: WORSE')
    sys.exit(rc)


if __name__ == '__main__':
    main()
