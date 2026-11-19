import re,sys,math
exec(open('ana/fm.py').read().split("paths=re.findall")[0].replace("s=open(sys.argv[1]).read()","s=open('cmp/out/dense-lr.flowmaid.svg').read()"))
paths=re.findall(r'<path d="([^"]*)"([^>]*)><title>([^<]*)</title>',s)
P={t:sample(d,200) for d,a,t in paths}
for name,(cx,cy,w) in {'nostrud':(1027.3,1566.3,60.8),'sed':(1058.7,2375.4,36.6),'tempor adipiscing':(998.7,2648.3,124.8),'tempor dolor':(307.5,892.5,93.1)}.items():
    ds=sorted((min(math.hypot(px-cx,py-cy) for px,py in pts),t) for t,pts in P.items())[:4]
    # horizontal distance at same y
    print(name,[(round(d,1),t) for d,t in ds])
    for t,pts in P.items():
        xs=[px for px,py in pts if abs(py-cy)<1.5]
        if xs and min(abs(x-cx) for x in xs)<120: print('   at y',cy,t,'x=',sorted(set(round(x) for x in xs))[:4])
