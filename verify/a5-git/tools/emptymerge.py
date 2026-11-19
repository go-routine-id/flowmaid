import sys,re
# report merges of branches with no own commits at merge time
for f in sys.argv[1:]:
    cur='main'; own={'main':0}; out=[]
    for line in open(f):
        t=line.strip()
        m=re.match(r'branch (\S+)',t)
        if m: own[m.group(1)]=0; cur=m.group(1); continue
        m=re.match(r'checkout (\S+)',t)
        if m: cur=m.group(1); continue
        m=re.match(r'merge (\S+)',t)
        if m:
            if own.get(m.group(1),0)==0: out.append(m.group(1))
            own[cur]=own.get(cur,0)+1; continue
        if t.startswith('commit'): own[cur]=own.get(cur,0)+1
    print(f, 'empty-branch merges:', out)
