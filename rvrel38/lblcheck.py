import re,sys
s=open(sys.argv[1]).read()
dirn=sys.argv[2] if len(sys.argv)>2 else 'TD'
paths=re.findall(r'<path d="(M [0-9][^"]*)"[^>]*stroke-width="[0-9.]+"',s)
segs=[]
for d in paths:
    cur=None
    for c,args in re.findall(r'([MLC])([^MLC]*)',d):
        v=list(map(float,re.findall(r'-?[0-9.]+',args)))
        p=(v[-2],v[-1])
        if c!='M' and cur: segs.append((cur,p))
        cur=p
# label chips: rect rx=4 then text
labs=re.findall(r'<rect x="([0-9.]+)" y="([0-9.]+)" width="([0-9.]+)" height="20" rx="4"[^>]*/>\s*<text x="[0-9.]+" y="[0-9.]+"[^>]*>([^<]*)',s)
for x,y,w,t in labs:
    cx=float(x)+float(w)/2; cy=float(y)+10
    on=[]
    for a,b in segs:
        if abs(a[1]-b[1])<0.2 and abs(a[1]-cy)<1 and min(a[0],b[0])-1<=cx<=max(a[0],b[0])+1: on.append('horiz')
        if abs(a[0]-b[0])<0.2 and abs(a[0]-cx)<1 and min(a[1],b[1])-1<=cy<=max(a[1],b[1])+1: on.append('vert')
    print(t, (round(cx,1),round(cy,1)), sorted(set(on)))
