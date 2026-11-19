#!/usr/bin/env python3
"""panics.py <base-outdir> <cand-outdir> [test-name ...]

Check 2: gate-message diffs between two suite-on test runs (trun.sh output, <outdir>/gates/{lib,quality}.txt),
not only failing test names. For every test (or only the named ones):
  status   NEWLY FAILING (ok -> FAILED) is worse; a baseline test the candidate no longer runs is MISSING (worse);
           NEWLY PASSING is better; a new candidate test that fails counts as NEWLY FAILING.
  message  the panic message of a test failing in both runs, line by line (thread ids, source line numbers and
           backtraces stripped). A candidate-only line is matched to a baseline-only line with the same text
           apart from its numbers (`bench-75`, `p01` stay text); the pair is WORSE when the first number that
           differs grew ("> ceiling", counts, lengths) or shrank on a "< floor" line, else BETTER. An unmatched
           candidate-only line is NEW (worse: a new violation row), an unmatched baseline-only line GONE (better).
  stdout   (informational) the captured stdout of every test that printed one in both runs: the gates' tables
           (quality_gate's TSV per file / mode / column, defect_baseline_gate's mode / field / value) and any
           other printed line, cell by cell; timing columns are skipped.
Exit status 0 when nothing is worse, 1 otherwise. --quiet leaves out the better and informational lines.
"""
import re
import sys

HEAD = re.compile(r'^---- (\S+) stdout ----$')
RESULT = re.compile(r'^test (\S+)(?: - should panic)? \.\.\. (ok|FAILED|ignored)\b')
PANIC = re.compile(r"^thread '([^']+)'(?: \(\d+\))? panicked at ([^:]+):\d+:\d+:\s*$")
NUM = re.compile(r'(?<![\w.\-/])-?\d+(?:\.\d+)?(?![\w.\-])')
TIMING = {'route_ms', 'scene_ms', 'drag_ms', 'drag_mean', 'drag_over'}
LOWER_WORSE = {'min_node_gap', 'min_edge_gap', 'chip_gap_min', 'lbl_gap_min'}


def parse(path):
    status, out, msg = {}, {}, {}
    cur, mode = None, None
    for raw in open(path, errors='replace'):
        line = raw.rstrip('\n')
        m = RESULT.match(line)
        if m and cur is None:
            status[m.group(1)] = m.group(2)
            continue
        h = HEAD.match(line)
        if h:
            cur, mode = h.group(1), 'out'
            out.setdefault(cur, []); msg.setdefault(cur, [])
            continue
        if cur is None:
            continue
        if line in ('failures:', 'successes:') or line.startswith('test result:'):
            cur = None
            continue
        p = PANIC.match(line)
        if p:
            mode = 'msg'
            msg[cur].append(f'panicked at {p.group(2)}')
            continue
        if line.startswith('stack backtrace:') or line.startswith('note: ') or re.match(r'^\s+\d+: ', line) or re.match(r'^\s+at ', line):
            continue
        (msg if mode == 'msg' else out)[cur].append(line)
    for d in (out, msg):
        for k in d:
            while d[k] and not d[k][-1].strip():
                d[k].pop()
    return status, out, msg


def load(outdir):
    status, out, msg = {}, {}, {}
    for f in ('lib.txt', 'quality.txt'):
        s, o, m = parse(f'{outdir}/gates/{f}')
        status.update(s); out.update(o); msg.update(m)
    return status, out, msg


def nums(line):
    return [float(x) for x in NUM.findall(line)]


def key(line):
    return NUM.sub('#', line)


def pair_dir(a, b):
    """+1 when line b is worse than line a (same key), -1 better, 0 same numbers"""
    na, nb = nums(a), nums(b)
    for x, y in zip(na, nb):
        if x != y:
            low = '< floor' in b or re.search(r'\d\s*<\s*\d', b) is not None
            return (1 if y < x else -1) if low else (1 if y > x else -1)
    return 0


def diff_lines(A, B):
    """multiset diff of two message line lists -> (worse, better) lists of strings"""
    a, b = list(A), list(B)
    for x in list(a):
        if x in b:
            a.remove(x); b.remove(x)
    worse, better = [], []
    for y in b:
        k = key(y)
        match = next((x for x in a if key(x) == k), None)
        if match is None:
            worse.append(f'+ NEW     {y}')
            continue
        a.remove(match)
        d = pair_dir(match, y)
        (worse if d >= 0 else better).append(f'{"~ WORSE " if d >= 0 else "~ better"} {match.strip()}  ->  {y.strip()}')
    for x in a:
        better.append(f'- gone    {x}')
    return worse, better


def cells_of(lines):
    """stdout lines -> {cell key: value string}; TSV tables with a 'file\\tmode' header become per-column cells"""
    cells, header, n = {}, None, {}
    for line in lines:
        f = line.split('\t')
        if len(f) > 2 and f[0] == 'file' and f[1] == 'mode':
            header = f
            continue
        if header and len(f) == len(header):
            for h, v in zip(header[2:], f[2:]):
                if h not in TIMING:
                    cells[f'{f[0]} {f[1]} {h}'] = v
            continue
        header = None
        if len(f) >= 2 and NUM.fullmatch(f[-1].strip() or 'x'):
            cells[' '.join(f[:-1])] = f[-1].strip()
            continue
        k = key(line)
        n[k] = n.get(k, 0) + 1
        cells[f'{k} #{n[k]}'] = ' '.join(NUM.findall(line)) or line
    return cells


def stdout_diff(A, B):
    ca, cb = cells_of(A), cells_of(B)
    rows = []
    for k in sorted(set(ca) | set(cb)):
        x, y = ca.get(k), cb.get(k)
        if x == y:
            continue
        tag = 'changed'
        try:
            fx, fy = float(x), float(y)
            col = k.split(' ')[-1]
            worse = (fy < fx) if col in LOWER_WORSE else (fy > fx)
            tag = 'higher' if fy > fx else 'lower'
            tag += ' (worse?)' if worse else ''
        except (TypeError, ValueError):
            pass
        rows.append(f'{k}: {x} -> {y}  {tag}')
    return rows


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    quiet = '--quiet' in sys.argv
    if len(args) < 2:
        print(__doc__)
        sys.exit(2)
    bdir, cdir, only = args[0], args[1], set(args[2:])
    try:
        sa, oa, ma = load(bdir)
        sb, ob, mb = load(cdir)
    except FileNotFoundError as e:
        print(f'panics: cannot read {e.filename}')
        sys.exit(1)
    names = sorted((set(sa) | set(sb)) if not only else only)
    worse_n = 0
    fa = sum(1 for v in sa.values() if v == 'FAILED'); fb = sum(1 for v in sb.values() if v == 'FAILED')
    print(f'[gates] tests {len(sa)} / {len(sb)}; failing {fa} -> {fb}')
    info = []
    for t in names:
        a, b = sa.get(t), sb.get(t)
        if a is None and b is None:
            print(f'  ??      {t}: not in either run'); worse_n += 1; continue
        if b is None:
            print(f'  MISSING {t} (was {a} in the baseline)'); worse_n += 1; continue
        if a != 'FAILED' and b == 'FAILED':
            print(f'  NEWLY FAILING {t}' + ('' if a else ' (new test)'))
            for line in mb.get(t, []):
                print(f'      {line}')
            worse_n += 1
            continue
        if a == 'FAILED' and b != 'FAILED':
            if not quiet:
                print(f'  newly passing {t}')
            continue
        if a is None and not quiet:
            info.append(f'  info new test {t}: {b}')
        if a == 'FAILED' and b == 'FAILED':
            w, bt = diff_lines(ma.get(t, []), mb.get(t, []))
            if w or (bt and not quiet):
                print(f'  == {t}: message lines worse {len(w)}, better {len(bt)}')
            for x in w:
                print(f'     {x}'); worse_n += 1
            if not quiet:
                for x in bt:
                    print(f'     {x}')
        if not quiet and oa.get(t) and ob.get(t):
            rows = stdout_diff(oa[t], ob[t])
            if rows:
                info.append(f'  info stdout {t}: {len(rows)} cells changed')
                info += [f'       {r}' for r in rows]
    for line in info:
        print(line)
    print('panics: no worse' if worse_n == 0 else f'panics: WORSE ({worse_n})')
    sys.exit(1 if worse_n else 0)


if __name__ == '__main__':
    main()
