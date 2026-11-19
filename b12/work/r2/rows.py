import sys, re
# rows.py <svg>: node labels grouped by y, sorted by x  (flowmaid dialect: <g ... data-id?>) fallback: text elements
s=open(sys.argv[1]).read()
nodes=re.findall(r'<g class="node[^"]*"[^>]*>.*?</g>', s, flags=re.S)
import xml.etree.ElementTree as ET
out={}
for m in re.finditer(r'<text[^>]*x="([-0-9.]+)"[^>]*y="([-0-9.]+)"[^>]*>(.*?)</text>', s, flags=re.S):
    x,y,t=float(m.group(1)),float(m.group(2)),re.sub(r'<[^>]+>',' ',m.group(3)).strip()
    out.setdefault(round(y/10),[]).append((x,t))
for y in sorted(out): print(" | ".join(t for _,t in sorted(out[y])))
