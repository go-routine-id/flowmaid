# Count crossings between ER relationship lines in flowmaid SVGs.
# usage: python3 ercross.py file.svg...
import re, sys, itertools
def parse(d):
    toks = re.findall(r'[MLC]|-?\d+\.?\d*', d)
    pts=[]; i=0; cur=None; segs=[]
    while i < len(toks):
        t=toks[i]
        if t=='M':
            cur=(float(toks[i+1]),float(toks[i+2])); pts=[cur]; i+=3
        elif t=='L':
            i+=1
            while i<len(toks) and toks[i] not in 'MLC':
                p=(float(toks[i]),float(toks[i+1])); pts.append(p); cur=p; i+=2
        elif t=='C':
            i+=1
            while i<len(toks) and toks[i] not in 'MLC':
                c1=(float(toks[i]),float(toks[i+1])); c2=(float(toks[i+2]),float(toks[i+3])); p=(float(toks[i+4]),float(toks[i+5])); i+=6
                for k in range(1,21):
                    t_=k/20; u=1-t_
                    pts.append((u**3*cur[0]+3*u*u*t_*c1[0]+3*u*t_*t_*c2[0]+t_**3*p[0], u**3*cur[1]+3*u*u*t_*c1[1]+3*u*t_*t_*c2[1]+t_**3*p[1]))
                cur=p
        else: i+=1
    return pts
def inter(a,b,c,d):
    def cr(o,p,q): return (p[0]-o[0])*(q[1]-o[1])-(p[1]-o[1])*(q[0]-o[0])
    d1=cr(c,d,a); d2=cr(c,d,b); d3=cr(a,b,c); d4=cr(a,b,d)
    return ((d1>0)!=(d2>0)) and ((d3>0)!=(d4>0)) and abs(d1)>1e-9 and abs(d2)>1e-9 and abs(d3)>1e-9 and abs(d4)>1e-9
