import re,sys
def load(f):
    s=open(f).read()
    out={}
    for m in re.finditer(r'<path d="([^"]*)"[^>]*><title>([^<]*)</title>',s):
        d,t=m.group(1),m.group(2)
        nums=[float(x) for x in re.findall(r'-?\d+\.?\d*',d)]
        xs=nums[0::2]; ys=nums[1::2]
        out.setdefault(t,[]).append((d,(min(xs),max(xs),min(ys),max(ys)),(xs[0],ys[0]),(xs[-1],ys[-1])))
    vb=re.search(r'viewBox="([^"]*)"',s).group(1)
    return out,vb
a,va=load(sys.argv[1]); b,vb=load(sys.argv[2])
print('viewBox',va,'|',vb)
for t in sys.argv[3:]:
    for x in a.get(t,[]): print('OLD',t,'bbox',x[1],'start',x[2],'end',x[3])
    for x in b.get(t,[]): print('NEW',t,'bbox',x[1],'start',x[2],'end',x[3])
