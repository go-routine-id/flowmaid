#!/usr/bin/env python3
# jd.py a.txt b.txt: must-fail lines only in a / only in b
import sys,re
def lines(p):
    out={}
    for l in open(p):
        if l.startswith('MUST FAIL'):
            f=l.split()
            out[(f[2],f[3])]=f[4]
    return out
a=lines(sys.argv[1]); b=lines(sys.argv[2])
for k in sorted(set(a)|set(b)):
    if a.get(k)!=b.get(k): print(f"{k[0]:36s} {k[1]:16s} {a.get(k,'ok'):>10s} -> {b.get(k,'ok')}")
