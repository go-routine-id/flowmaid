import re,sys
for f in sys.argv[1:]:
    s=open(f).read()
    print("==",f.split('/')[-1])
    # mermaid node groups: <g class="node ..." id="state-A-1" transform="translate(x, y)">
    for m in re.finditer(r'<g[^>]*class="([^"]*)"[^>]*id="([^"]*)"[^>]*transform="translate\(([-\d.]+),\s*([-\d.]+)\)"',s):
        cls,id_,x,y=m.groups()
        if 'node' in cls or 'cluster' in cls or 'root' in cls:
            print("  %-40s x=%8s y=%8s  [%s]"%(id_,x,y,cls[:30]))
    for m in re.finditer(r'<g[^>]*id="([^"]*)"[^>]*class="([^"]*)"[^>]*transform="translate\(([-\d.]+),\s*([-\d.]+)\)"',s):
        id_,cls,x,y=m.groups()
        if 'node' in cls or 'cluster' in cls or 'root' in cls:
            print("  %-40s x=%8s y=%8s  [%s]"%(id_,x,y,cls[:30]))
    for m in re.finditer(r'<g class="root" transform="translate\(([-\d.]+),\s*([-\d.]+)\)"',s):
        print("  root translate",m.groups())
