#!/usr/bin/env python3
"""Fan-ins of n classes into one hub, every mix of cardinalities and labels.
usage: gen2.py <outdir> <n>..."""
import os
import sys

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
OPS = {
    'arrow': ('-->', '<--'), 'inherit': ('<|--', '--|>'), 'comp': ('*--', '--*'), 'aggr': ('o--', '--o'),
    'link': ('--', '--'), 'dep': ('..>', '<..'), 'real': ('..|>', '<|..'), 'dash': ('..', '..'),
}
DIRS = ['TB', 'BT', 'LR', 'RL']
CS = {'n': None, '1': '1', 'r': '0..*', 'm': 'many'}
CT = {'n': None, 's': '*', 'r': '1..*', 'm': 'many'}
LB = {'n': None, 'u': 'uses%d', 'o': 'owns', 'l': 'is notified by'}
names = []
for n in [int(a) for a in sys.argv[2:]]:
    for d in DIRS:
        for opn, (op, rop) in OPS.items():
            for order in 'AB':
                for csn, cs in CS.items():
                    for ctn, ct in CT.items():
                        for lbn, lb in LB.items():
                            L = ['classDiagram', '  direction ' + d]
                            for i in range(n):
                                if order == 'A':
                                    a, b, o, ca, cb = 'Svc%d' % i, 'Hub', op, cs, ct
                                else:
                                    a, b, o, ca, cb = 'Hub', 'Svc%d' % i, rop, ct, cs
                                s = '  ' + a
                                if ca:
                                    s += ' "%s"' % ca
                                s += ' ' + o
                                if cb:
                                    s += ' "%s"' % cb
                                s += ' ' + b
                                if lb:
                                    s += ' : ' + (lb % i if '%' in lb else lb)
                                L.append(s)
                            nm = '%s_n%02d_%s_%s_c%s%s_l%s' % (order, n, d, opn, csn, ctn, lbn)
                            with open(os.path.join(out, nm + '.mmd'), 'w') as f:
                                f.write('\n'.join(L) + '\n')
                            names.append(nm)
with open(os.path.join(out, 'LIST'), 'w') as f:
    f.write('\n'.join(names) + '\n')
print(len(names))
