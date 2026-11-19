import re,sys
s=open(sys.argv[1],encoding='utf-8').read()
body=re.sub(r'```.*?```','',s,flags=re.S)
heads=[]
for m in re.finditer(r'^(#{1,6})\s+(.*)$',body,flags=re.M):
    t=m.group(2).strip()
    a=re.sub(r'[^\w\- ]','',t.lower()).replace(' ','-')
    heads.append(a)
links=set(re.findall(r'\]\(#([^)]+)\)',body))
for l in sorted(links):
    print(('ok  ' if l in heads else 'MISSING ')+l)
