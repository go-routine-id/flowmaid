import re,sys
def parse(p):
    s=open(p).read()
    edges={}
    for m in re.finditer(r'<path([^>]*)>\s*<title>([^<]*)</title>',s):
        d=re.search(r' d="([^"]*)"',m.group(1))
        if d: edges[m.group(2)]=d.group(1)
    labels=re.findall(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="20" rx="4"[^>]*/>\s*<text[^>]*>([^<]*)</text>',s)
    return edges,{l[3]:(float(l[0]),float(l[1])) for l in labels}
for f in sys.argv[2:]:
    a,la=parse(sys.argv[1]+'/'+f); b,lb=parse('new/'+f)
    ch=[k for k in b if a.get(k)!=b[k]]
    lm=[(k,la.get(k),lb[k]) for k in lb if la.get(k)!=lb[k]]
    print(f, 'edges changed:', ch, 'labels moved:', lm)
