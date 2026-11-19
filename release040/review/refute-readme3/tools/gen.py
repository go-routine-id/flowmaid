#!/usr/bin/env python3
"""Generate fan-in class diagrams: n classes each with one relation to a hub."""
import os
import sys

out = sys.argv[1]
os.makedirs(out, exist_ok=True)

OPS = {
    'arrow': '-->', 'inherit': '<|--', 'comp': '*--', 'aggr': 'o--',
    'link': '--', 'dep': '..>', 'real': '..|>', 'dash': '..',
}
# the same eight written the other way round (hub on the left)
ROPS = {
    'arrow': '<--', 'inherit': '--|>', 'comp': '--*', 'aggr': '--o',
    'link': '--', 'dep': '<..', 'real': '<|..', 'dash': '..',
}
DIRS = ['TB', 'BT', 'LR', 'RL']
names = []


def rel(a, b, op, variant, i):
    if variant == 'plain':
        return '  %s %s %s' % (a, op, b)
    if variant == 'label':
        return '  %s %s %s : uses%d' % (a, op, b, i)
    if variant == 'card':
        return '  %s "1" %s "*" %s : uses%d' % (a, op, b, i)
    if variant == 'cardonly':
        return '  %s "1" %s "0..*" %s' % (a, op, b)
    if variant == 'card1':
        return '  %s %s "many" %s : owns' % (a, op, b)
    if variant == 'longlabel':
        return '  %s "1..*" %s "0..1" %s : %s' % (a, op, b, ['reads', 'writes to the', 'owns', 'is notified by', 'has'][i % 5])
    raise SystemExit(variant)


def write(name, lines):
    with open(os.path.join(out, name + '.mmd'), 'w') as f:
        f.write('\n'.join(lines) + '\n')
    names.append(name)


for n in range(6, 13):
    for d in DIRS:
        for variant in ['plain', 'label', 'card', 'cardonly', 'card1', 'longlabel']:
            for opn, op in OPS.items():
                # A: Svc op Hub (the reviewer's family for plain/label/card)
                L = ['classDiagram', '  direction ' + d]
                L += [rel('Svc%d' % i, 'Hub', op, variant, i) for i in range(n)]
                write('A_n%02d_%s_%s_%s' % (n, d, variant, opn), L)
                # B: Hub op' Svc (hub written first)
                L = ['classDiagram', '  direction ' + d]
                L += [rel('Hub', 'Svc%d' % i, ROPS[opn], variant, i) for i in range(n)]
                write('B_n%02d_%s_%s_%s' % (n, d, variant, opn), L)
            # E: the eight operators mixed
            ops = list(OPS.values())
            L = ['classDiagram', '  direction ' + d]
            L += [rel('Svc%d' % i, 'Hub', ops[i % 8], variant, i) for i in range(n)]
            write('E_n%02d_%s_%s_mixed' % (n, d, variant), L)
            # D: boxes of different sizes (members, long names)
            long_names = ['OrderService', 'Cart', 'PaymentGatewayAdapter', 'User', 'InventoryRepository',
                          'Mailer', 'AuditLog', 'ShippingCostCalculator', 'Api', 'SessionStore', 'Cache', 'Scheduler']
            for opn in ['arrow', 'inherit', 'comp', 'dep']:
                op = OPS[opn]
                L = ['classDiagram', '  direction ' + d]
                L += ['  class Hub {', '    +id int', '    +name String', '    +save() bool', '  }']
                for i in range(n):
                    if i % 3 == 0:
                        L += ['  class %s {' % long_names[i], '    +run()', '    -state int', '  }']
                    elif i % 3 == 1:
                        L += ['  class %s' % long_names[i]]
                for i in range(n):
                    L.append(rel(long_names[i], 'Hub', op, variant, i))
                write('D_n%02d_%s_%s_%s' % (n, d, variant, opn), L)

with open(os.path.join(out, 'LIST'), 'w') as f:
    f.write('\n'.join(names) + '\n')
print(len(names))
