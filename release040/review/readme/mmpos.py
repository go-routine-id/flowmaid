import re,sys
# mermaid state svg: nodes are <g class="node ... " id="state-NAME-n" transform="translate(x, y)">
for f in sys.argv[1:]:
    s=open(f).read()
    out=[]
    for m in re.finditer(r'<g class="([^"]*)" id="([^"]*)"[^>]*?transform="translate\(([\-\d.]+),\s*([\-\d.]+)\)"',s):
        out.append(f"{m.group(2)}@({float(m.group(3)):.0f},{float(m.group(4)):.0f})")
    # clusters
    print(f.split('/')[-1], ' '.join(out))
