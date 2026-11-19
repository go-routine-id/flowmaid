#!/usr/bin/env python3
"""Render every generated input with one flowmaid binary and count crossings.
usage: run.py <binary> <indir> <out.tsv>"""
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import xcount  # noqa: E402

binary, indir, outp = sys.argv[1:4]
names = open(os.path.join(indir, 'LIST')).read().split()
with open(outp, 'w') as out:
    for nm in names:
        src = os.path.join(indir, nm + '.mmd')
        p = subprocess.run([binary, src], capture_output=True, text=True)
        if p.returncode != 0 or not p.stdout.startswith('<svg'):
            out.write('%s\tERR\t%s\n' % (nm, p.stderr.strip().replace('\n', ' ')[:200]))
            continue
        nrel = sum(1 for l in open(src) if ('Hub' in l and ('--' in l or '..' in l)))
        n, pairs, total, ov, detail, cmds = xcount.count(p.stdout)
        out.write('%s\t%d\t%d\t%d\t%d\t%.1f\t%s\n' % (nm, nrel, n, pairs, total, ov, ''.join(sorted(cmds))))
