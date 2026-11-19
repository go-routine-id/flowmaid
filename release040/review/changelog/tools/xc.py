#!/usr/bin/env python3
# count edge curves and crossings in a flowmaid class SVG
import re,sys
def parse(d):
    toks=re.findall(r'[MLCQZ]|-?\d+\.?\d*(?:e-?\d+)?',d)
    i=0;pts=[];cur=None;cmds=[]
    while i<len(toks):
        t=toks[i]
        if t=='M':
            cur=(float(toks[i+1]),float(toks[i+2]));pts.append(cur);i+=3;cmds.append('M')
        elif t=='L':
            i+=1
            while i<len(toks) and toks[i] not in 'MLCQZ':
                cur=(float(toks[i]),float(toks[i+1]));pts.append(cur);i+=2
            cmds.append('L')
        elif t=='C':
            i+=1
            while i<len(toks) and toks[i] not in 'MLCQZ':
                p1=(float(toks[i]),float(toks[i+1]));p2=(float(toks[i+2]),float(toks[i+3]));p3=(float(toks[i+4]),float(toks[i+5]));i+=6
                p0=cur
                for k in range(1,25):
                    u=k/24.0;a=(1-u)**3;b=3*(1-u)**2*u;c=3*(1-u)*u*u;e=u**3
                    pts.append((a*p0[0]+b*p1[0]+c*p2[0]+e*p3[0],a*p0[1]+b*p1[1]+c*p2[1]+e*p3[1]))
                cur=p3
                cmds.append('C')
        elif t=='Q':
            i+=1
            while i<len(toks) and toks[i] not in 'MLCQZ':
                p1=(float(toks[i]),float(toks[i+1]));p2=(float(toks[i+2]),float(toks[i+3]));i+=4
                p0=cur
                for k in range(1,13):
                    u=k/12.0
                    pts.append(((1-u)**2*p0[0]+2*(1-u)*u*p1[0]+u*u*p2[0],(1-u)**2*p0[1]+2*(1-u)*u*p1[1]+u*u*p2[1]))
                cur=p2
                cmds.append('Q')
        else:
            i+=1;cmds.append(t)
    return pts,cmds
def seg_x(a,b,c,d):
    def cr(o,p,q): return (p[0]-o[0])*(q[1]-o[1])-(p[1]-o[1])*(q[0]-o[0])
    d1=cr(c,d,a);d2=cr(c,d,b);d3=cr(a,b,c);d4=cr(a,b,d)
    if ((d1>0 and d2<0) or (d1<0 and d2>0)) and ((d3>0 and d4<0) or (d3<0 and d4>0)):
        t=d1/(d1-d2)
        return (a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1]))
    return None
def edges(svg):
    out=[]
    for m in re.finditer(r'<path d="([^"]*)"([^>]*)>',svg):
        d,rest=m.group(1),m.group(2)
        if 'fill="none"' not in rest: continue
        if ' C ' not in d: continue
        pts,cmds=parse(d)
        out.append((d,pts,cmds))
    return out
def measure(svg):
    es=edges(svg)
    curves=0
    for d,pts,cmds in es:
        # a plain curve: M + exactly one C (no L), not straight
        if cmds==['M','C']:
            xs=set(round(p[0],1) for p in pts); ys=set(round(p[1],1) for p in pts)
            if len(xs)>1 and len(ys)>1: curves+=1
    cross=0
    for i in range(len(es)):
        for j in range(i+1,len(es)):
            A=es[i][1];Bp=es[j][1]
            hits=[]
            for k in range(len(A)-1):
                a,b=A[k],A[k+1]
                for l in range(len(Bp)-1):
                    p=seg_x(a,b,Bp[l],Bp[l+1])
                    if p:
                        if not any(abs(p[0]-h[0])+abs(p[1]-h[1])<2.0 for h in hits): hits.append(p)
            cross+=len(hits)
    return len(es),curves,cross
if __name__=='__main__':
    for f in sys.argv[1:]:
        try:
            s=open(f).read()
            n,c,x=measure(s)
            print(f"{f}\t{n}\t{c}\t{x}")
        except Exception as e:
            print(f"{f}\tERR {e}")
