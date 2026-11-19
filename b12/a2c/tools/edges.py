#!/usr/bin/env python3
# edges.py file.svg [substr...]: each edge path's title and its corner points (curve control points dropped)
import re,sys
s=open(sys.argv[1]).read()
flt=sys.argv[2:]
for m in re.finditer(r'<path d="([^"]*)"[^>]*>(?:<title>([^<]*)</title>)?',s):
    d,t=m.group(1),m.group(2) or ''
    if not t: continue
    if flt and not any(f in t for f in flt): continue
    toks=re.findall(r'[MLCAQ]|-?[\d.]+',d)
    pts=[];i=0;cmd=None
    while i<len(toks):
        if toks[i] in 'MLCAQ': cmd=toks[i]; i+=1; continue
        if cmd=='C':
            x,y=float(toks[i+4]),float(toks[i+5]); i+=6
        elif cmd=='A':
            x,y=float(toks[i+5]),float(toks[i+6]); i+=7
        elif cmd=='Q':
            x,y=float(toks[i+2]),float(toks[i+3]); i+=4
        else:
            x,y=float(toks[i]),float(toks[i+1]); i+=2
        if not pts or abs(pts[-1][0]-x)>0.05 or abs(pts[-1][1]-y)>0.05: pts.append((x,y))
    print(f"{t:28s} "+" ".join(f"({x:.1f},{y:.1f})" for x,y in pts))
