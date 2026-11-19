#!/usr/bin/env python3
"""qdiff.py A.tsv B.tsv [cols]: per file/mode, print the listed quality columns that differ."""
import sys
cols = (sys.argv[3].split(',') if len(sys.argv) > 3 else
        'defects,x_drawn,lateral_x,fan_x,edge_chip,side_ends,min_edge_gap,lbl_edge,lbl_crowd,chip_gap_min,lat_wiggles,bends_max,sib_stagger,edge_len,width,height,compact_max'.split(','))
def load(f):
    rows = {}
    head = None
    for l in open(f):
        c = l.rstrip('\n').split('\t')
        if c[0] == 'file':
            head = c
            continue
        if head and len(c) == len(head):
            rows[(c[0], c[1])] = dict(zip(head, c))
    return rows
a, b = load(sys.argv[1]), load(sys.argv[2])
for k in sorted(set(a) | set(b)):
    if k not in a or k not in b:
        print(k, 'missing in', 'A' if k not in a else 'B')
        continue
    d = [f'{c} {a[k].get(c)}->{b[k].get(c)}' for c in cols if a[k].get(c) != b[k].get(c)]
    if d:
        print(f'{k[0]:28} {k[1]:7}', '; '.join(d))
