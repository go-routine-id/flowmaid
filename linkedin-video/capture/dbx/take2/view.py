import sys
f, c0, c1 = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
r0 = int(sys.argv[4]) if len(sys.argv) > 4 else 0
r1 = int(sys.argv[5]) if len(sys.argv) > 5 else 10**9
for i, l in enumerate(open(f, encoding='utf-8').read().split('\n')):
    if r0 <= i < r1:
        print('%3d %s' % (i, l[c0:c1].rstrip()))
