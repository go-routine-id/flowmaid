import re,sys
for f in sys.argv[1:]:
    s=open(f).read()
    out=[]
    # mermaid: <g class="node ..." id="state-A-1" transform="translate(x, y)">
    for m in re.finditer(r'<g[^>]*class="[^"]*node[^"]*"[^>]*>', s):
        tag=m.group(0)
        i=re.search(r'id="([^"]+)"',tag); t=re.search(r'transform="translate\(([-\d.]+)[ ,]+([-\d.]+)\)"',tag)
        if i and t: out.append((i.group(1), float(t.group(1)), float(t.group(2))))
    print(f.split('/')[-1])
    for o in out: print("   %-28s x=%7.1f y=%7.1f"%o)
