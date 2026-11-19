import re,sys
for f in sys.argv[1:]:
    s=open(f).read()
    out=[]
    for m in re.finditer(r'<g><title>([^<]*)</title>\s*<(rect|circle|path|polygon)[^>]*?(?:x="([\d.\-]+)" y="([\d.\-]+)"|cx="([\d.\-]+)" cy="([\d.\-]+)")',s):
        t=m.group(1); x=m.group(3) or m.group(5); y=m.group(4) or m.group(6)
        out.append(f"{t}@({x},{y})")
    print(f.split('/')[-1], ' '.join(out))
