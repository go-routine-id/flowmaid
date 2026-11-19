import re,sys
for f in sys.argv[1:]:
    n=0; left=[]
    for l in open(f):
        if not l.startswith('E'): continue
        m=re.match(r'E(\d+) (\S+)->(\S+) label=(.*) bez=\[(.*?)\] wps=\[(.*)\]',l.strip())
        if m.group(6): continue
        p=[tuple(map(float,q.strip('()').split(','))) for q in m.group(5).split()]
        # vertical or horizontal handles
        for ax in (0,1):
            o=1-ax
            if p[1][ax]==p[0][ax] and p[2][ax]==p[3][ax] and p[1][o]!=p[0][o]:
                off=abs(p[3][ax]-p[0][ax])
                if 1e-6<off<28: left.append((m.group(1),m.group(2),m.group(3),round(off,2)))
                break
        n+=1
    print(f,'single cubics',n,'S under 28 left',len(left),left[:8])
