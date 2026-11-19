# hl.py <in.svg> <out.svg> <from>to>...: stroke the named edges red and thick (edge = path with <title>from → to...)
import sys,re
src=open(sys.argv[1]).read(); want=[w.split('>') for w in sys.argv[3:]]
def repl(m):
    path=m.group(0)
    t=re.search(r'<title>([^<]*)</title>',path)
    if not t: return path
    title=t.group(1).replace('&gt;','>')
    for f,to in want:
        if re.match(r'\s*'+re.escape(f)+r'\s*(→|&#8594;)\s*'+re.escape(to)+r'(\s|:|$)',title):
            path=re.sub(r'stroke="[^"]*"','stroke="#e8000b"',path,count=1)
            path=re.sub(r'stroke-width="[^"]*"','stroke-width="4"',path,count=1)
    return path
out=re.sub(r'<path d="[^"]*"[^>]*>\s*<title>[^<]*</title>',repl,src)
open(sys.argv[2],'w').write(out)
