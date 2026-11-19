import re,sys,collections
t=open(sys.argv[1],encoding='utf-8').read()
heads=[]
incode=False
for l in t.split('\n'):
    if l.startswith('```'): incode=not incode
    if not incode and re.match(r'^#{1,6} ',l):
        h=re.sub(r'^#+ ','',l).strip().lower()
        h=re.sub(r'[^\w\- ]','',h).replace(' ','-')
        heads.append(h)
links=re.findall(r'\]\(#([^)]+)\)',t)
c=collections.Counter(links)
bad=[a for a in c if a not in heads]
print('bad anchors:',bad)
print('link counts:',sorted(c.items(),key=lambda x:-x[1])[:25])
