import re,sys
p=sys.argv[1]
lines=open(p,encoding='utf-8').read().split('\n')
heads={}
incode=False
order=[]
for i,l in enumerate(lines,1):
    if l.strip().startswith('```'):
        incode=not incode; continue
    if incode: continue
    m=re.match(r'^(#{1,6})\s+(.*?)\s*#*\s*$',l)
    if m:
        t=m.group(2)
        # strip markdown: links, code ticks, emphasis
        t2=re.sub(r'\[([^\]]*)\]\([^)]*\)',r'\1',t)
        t2=t2.replace('`','')
        t2=re.sub(r'[*_]{1,2}([^*_]+)[*_]{1,2}',r'\1',t2) if False else t2
        s=t2.strip().lower()
        s=re.sub(r'[^\w\- ]','',s,flags=re.U)
        s=s.replace(' ','-')
        base=s;n=0
        while s in heads:
            n+=1; s=f'{base}-{n}'
        heads[s]=(i,t)
        order.append((i,m.group(1),t,s))
incode=False
bad=0;cnt=0
for i,l in enumerate(lines,1):
    if l.strip().startswith('```'):
        incode=not incode; continue
    if incode: continue
    for m in re.finditer(r'\]\(#([^)]+)\)',l):
        cnt+=1
        a=m.group(1)
        if a not in heads:
            bad+=1
            print('BROKEN',i,a)
print('links',cnt,'broken',bad,'headings',len(heads))
if len(sys.argv)>2:
    for o in order: print(o)
