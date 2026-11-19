import re,sys,math
svg=open(sys.argv[1]).read()
svg=re.sub(r'<mask .*?</mask>','',svg,flags=re.S)
nodes={}
for m in re.finditer(r'<g><title>([^<]*)</title>\s*<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"',svg):
    nodes[m.group(1)]=tuple(map(float,m.groups()[1:]))
edges=[]
for m in re.finditer(r'<path d="([^"]*)"[^>]*?(?:mask="url\(#([^)]*)\)")?[^>]*><title>([^<]*)</title>',svg):
    d,mask,t=m.groups()
    edges.append((t,d,mask))
print(len(nodes),'nodes',len(edges),'edges')
def ctr(n):
    x,y,w,h=nodes[n];return x+w/2,y+h/2
cnt={'R':0,'L':0,'D':0,'U':0}; xfwd=0
kinds={}
for t,d,mask in edges:
    a,b=[s.strip() for s in t.split('→')]
    if a=='external api': pass
    na = 'external api' if a=='ext' else a
    nb = 'external api' if b=='ext' else b
    (x1,y1),(x2,y2)=ctr(na),ctr(nb)
    dx,dy=x2-x1,y2-y1
    if abs(dx)>=abs(dy): k='R' if dx>0 else 'L'
    else: k='D' if dy>0 else 'U'
    cnt[k]+=1
    if dx>0: xfwd+=1
    cmds=re.findall(r'[MLCQAZ]',d)
    nC=cmds.count('C'); nL=cmds.count('L')
    kind = 'single-cubic' if nC==1 and nL==0 else ('poly' if nC==0 else 'mixed')
    kinds.setdefault(kind,[]).append(t)
print(cnt,'target x > source x:',xfwd)
for k,v in kinds.items(): print(k,len(v))
print('single-cubic:',kinds.get('single-cubic'))
