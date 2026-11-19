#!/usr/bin/env python3
"""List or select hunks of `git diff` output.
usage: hunks.py list <diff>            -> prints idx, file, first changed lines
       hunks.py pick <diff> <idx,...>  -> prints a patch with only those hunks
"""
import sys, re
def parse(path):
    files = []  # (header_lines, [hunk_lines...])
    cur = None
    for line in open(path, encoding='utf-8'):
        if line.startswith('diff --git'):
            cur = [[line], []]; files.append(cur)
        elif cur is not None and not cur[1] and not line.startswith('@@'):
            cur[0].append(line)
        elif line.startswith('@@'):
            cur[1].append([line])
        else:
            cur[1][-1].append(line)
    return files
mode, path = sys.argv[1], sys.argv[2]
files = parse(path)
idx = 0
flat = []
for f in files:
    for h in f[1]:
        flat.append((f, h)); 
if mode == 'list':
    for i, (f, h) in enumerate(flat):
        name = f[0][0].split()[-1]
        ch = [l.rstrip() for l in h[1:] if l[:1] in '+-'][:3]
        print(i, name, h[0].strip()[:40], '|', ' / '.join(c[:70] for c in ch))
else:
    pick = set(int(x) for x in sys.argv[3].split(','))
    out = []
    for f in files:
        hs = [h for h in f[1] if flat.index((f, h)) in pick]
        if hs:
            out.extend(f[0]); [out.extend(h) for h in hs]
    sys.stdout.write(''.join(out))
