#!/usr/bin/env python3
"""qcols.py <regex-files> <tsv>... : static rows of the named files, verdict columns, one row per tsv"""
import sys, re
pat = re.compile(sys.argv[1]); cols = ['defects','x_drawn','lateral_x','fan_x','edge_chip','side_ends','min_edge_gap','overlap_pairs','tight_pairs','edge_frame_x']
for t in sys.argv[2:]:
    L = [l.rstrip('\n').split('\t') for l in open(t)]
    h = L[0]; ix = [h.index(c) for c in cols]
    for r in L[1:]:
        if len(r) > 2 and r[1] == 'static' and pat.search(r[0]):
            print('%-22s %-28s' % (t.split('/')[-2][:22], r[0]), ' '.join('%s=%s' % (c, r[i]) for c, i in zip(cols, ix)))
