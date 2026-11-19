import sys
def lum(c):
    c=c.lstrip('#')
    if len(c)==3: c=''.join(x*2 for x in c)
    r,g,b=[int(c[i:i+2],16)/255 for i in (0,2,4)]
    f=lambda v: v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4
    return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b)
def cr(a,b):
    la,lb=lum(a),lum(b); return (max(la,lb)+0.05)/(min(la,lb)+0.05)
def scale(c,k):
    c=c.lstrip('#'); return '#'+''.join('%02x'%min(255,round(int(c[i:i+2],16)*k)) for i in (0,2,4))
def mixw(c,t):
    c=c.lstrip('#'); return '#'+''.join('%02x'%round(int(c[i:i+2],16)+(255-int(c[i:i+2],16))*t) for i in (0,2,4))
if __name__=='__main__':
    a=sys.argv[1:]
    for i in range(0,len(a),2): print(a[i],a[i+1],round(cr(a[i],a[i+1]),2))
