import re,sys
def load(f):
    out={}
    for l in open(f):
        if not l.startswith('E'): continue
        m=re.match(r'E(\d+) (\S+)->(\S+) label=(.*) bez=\[(.*?)\] wps=\[(.*)\]',l.strip())
        i=int(m.group(1))
        pts=lambda s:[tuple(map(float,p.strip('()').split(','))) for p in s.split()] if s else []
        out[i]=(m.group(2),m.group(3),pts(m.group(5)),pts(m.group(6)),m.group(4))
    return out
a=load(sys.argv[1]); b=load(sys.argv[2])
from collections import Counter
c=Counter()
for i in a:
    fa,ta,ba,wa,_=a[i]; fb,tb,bb,wb,_=b[i]
    if not wa and wb:
        off=abs(ba[3][0]-ba[0][0])
        c['new wps']+=1
        if len(sys.argv)>3: print(i,fa,ta,'old off across %.2f'%off,'old bez',ba,'new nwps',len(wb), 'new corners?', wb)
    elif not wa and not wb and ba!=bb:
        offa=abs(ba[3][0]-ba[0][0]); offb=abs(bb[3][0]-bb[0][0])
        c['bez changed']+=1
        if len(sys.argv)>3: print(i,'bez changed off %.2f -> %.2f'%(offa,offb))
    elif wa and not wb: c['lost wps']+=1
    elif wa==wb and ba==bb: c['same']+=1
    else: c['wps changed']+=1
print(dict(c))
