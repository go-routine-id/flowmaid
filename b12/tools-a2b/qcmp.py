#!/usr/bin/env python3
"""qcmp.py <base-outdir> <cand-outdir>

Check 3: the 332 + 32 direction copies (qrun.sh output, <outdir>/dirs/{dirs,dirsx}.tsv), static and routed, copy by
copy. Columns are the harness's (`flowmaid::quality::Metrics`, "lower is better" unless noted).
Verdict columns (the skeptic's set, critique.md 1.3):
  defects, x_drawn, lateral_x, fan_x, edge_chip, side_ends   a copy is worse when its count grows
  min_edge_gap                                               a copy is worse when it drops by more than 0.5 px
  and a copy (file, mode) the candidate no longer measures is worse.
Every other column is informational: per mode the total and how many copies got worse / better (higher is worse,
except min_node_gap / chip_gap_min; descriptive columns such as nodes, width, ink and the idiom counts carry no
direction; timings are skipped).
Exit status 0 when no verdict column is worse on any copy, 1 otherwise. --quiet leaves out the informational part.
"""
import csv
import sys

VCOLS = ['defects', 'x_drawn', 'lateral_x', 'fan_x', 'edge_chip', 'side_ends']
GAP = 'min_edge_gap'
GAP_TOL = 0.5
SKIP = {'route_ms', 'scene_ms', 'drag_ms', 'drag_mean', 'drag_over', 'deterministic', 'dfr_own', 'dfr_far'}
LOWER_WORSE = {'min_node_gap', 'min_edge_gap', 'chip_gap_min'}
NEUTRAL = {'nodes', 'edges', 'frames', 'width', 'height', 'aspect', 'area_per_node', 'center_off', 'ink',
           'idiom_ortho', 'idiom_scubic', 'idiom_bow', 'ortho_edges', 'xhops', 'block_mode'}
FAMS = ['dirs', 'dirsx']


def load(outdir, fam):
    rows = list(csv.reader(open(f'{outdir}/dirs/{fam}.tsv'), delimiter='\t'))
    head = rows[0]
    out = {}
    for r in rows[1:]:
        if not r or r[0] == 'TOTAL':
            continue
        out[(fam, r[0], r[1])] = {h: float(v) for h, v in zip(head[2:], r[2:])}
    return head[2:], out


def g(x):
    return ('%g' % x) if abs(x) < 1e7 else '%.0f' % x


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    quiet = '--quiet' in sys.argv
    if len(args) != 2:
        print(__doc__)
        sys.exit(2)
    worse_n = 0
    for fam in FAMS:
        try:
            cols, A = load(args[0], fam)
            _, Bm = load(args[1], fam)
        except FileNotFoundError as e:
            print(f'[{fam}] cannot read {e.filename}')
            worse_n += 1
            continue
        missing = sorted(k for k in A if k not in Bm)
        for k in missing:
            print(f'[{fam}] MISSING {k[1]} {k[2]}')
        worse_n += len(missing)
        n = len({k[1] for k in A})
        for mode in ['static', 'routed']:
            keys = sorted(k for k in A if k[2] == mode and k in Bm)
            parts, lists = [], []
            for c in VCOLS:
                ta = sum(A[k][c] for k in keys); tb = sum(Bm[k][c] for k in keys)
                w = [f'{k[1]} {g(A[k][c])}->{g(Bm[k][c])}' for k in keys if Bm[k][c] > A[k][c]]
                b = sum(1 for k in keys if Bm[k][c] < A[k][c])
                parts.append(f'{c} {g(ta)}->{g(tb)} (+{len(w)}/-{b})')
                if w:
                    lists.append(f'    WORSE {c} ({len(w)}): ' + '; '.join(w))
                worse_n += len(w)
            gap = [f'{k[1]} {g(A[k][GAP])}->{g(Bm[k][GAP])}' for k in keys if Bm[k][GAP] < A[k][GAP] - GAP_TOL]
            parts.append(f'{GAP} down {len(gap)}')
            if gap:
                lists.append(f'    WORSE {GAP} drop > {GAP_TOL} px ({len(gap)}): ' + '; '.join(gap))
            worse_n += len(gap)
            print(f'[{fam} {mode}] {n} copies  ' + '  '.join(parts))
            for line in lists:
                print(line)
            if quiet:
                continue
            info = []
            for c in cols:
                if c in SKIP or c in VCOLS or c == GAP:
                    continue
                ta = sum(A[k][c] for k in keys); tb = sum(Bm[k][c] for k in keys)
                up = [k for k in keys if Bm[k][c] > A[k][c] + 1e-9]
                dn = [k for k in keys if Bm[k][c] < A[k][c] - 1e-9]
                if not up and not dn:
                    continue
                if c in NEUTRAL:
                    tag = 'no direction'
                    bad = []
                else:
                    bad = dn if c in LOWER_WORSE else up
                    tag = f'worse on {len(bad)}, better on {len(dn) if bad is up else len(up)}'
                ex = '; '.join(f'{k[1]} {g(A[k][c])}->{g(Bm[k][c])}' for k in bad[:6]) + (' ...' if len(bad) > 6 else '')
                info.append(f'    info {c}: total {g(ta)}->{g(tb)}, up {len(up)} / down {len(dn)} ({tag})' + (f': {ex}' if ex else ''))
            for line in info:
                print(line)
    print('dirs: no worse' if worse_n == 0 else f'dirs: WORSE ({worse_n})')
    sys.exit(1 if worse_n else 0)


if __name__ == '__main__':
    main()
