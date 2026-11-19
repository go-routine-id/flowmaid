import re, collections
NEW='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/wt/rel39/README.md'
OLD='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/compare/README.pre-trim-039.md'
new=open(NEW,encoding='utf-8').read()
old=open(OLD,encoding='utf-8').read()
nl=new.split('\n'); ol=old.split('\n')
# table col counts
def cells(l):
    # split on unescaped pipes outside backticks
    out=[];cur='';i=0;tick=0
    s=l.strip()
    # handle code spans of varying length
    while i<len(s):
        c=s[i]
        if c=='`':
            j=i
            while j<len(s) and s[j]=='`': j+=1
            n=j-i
            if tick==0: tick=n
            elif tick==n: tick=0
            cur+=s[i:j]; i=j; continue
        if c=='\\' and i+1<len(s): cur+=s[i:i+2]; i+=2; continue
        if c=='|' and tick==0:
            out.append(cur); cur=''; i+=1; continue
        cur+=c; i+=1
    out.append(cur)
    return out
fence=False; tbl=[]; 
for i,l in enumerate(nl,1):
    if l.startswith('```'): fence=not fence
    if fence: continue
    if l.startswith('|'):
        tbl.append((i,l))
    else:
        if tbl:
            ns=[len(cells(x))-2 for _,x in tbl]
            ns2=[x.count('|')-1 for _,x in tbl]
            print('table at',tbl[0][0],'rows',len(tbl),'cols(codeaware)',sorted(set(ns)),'cols(raw pipes)',sorted(set(ns2)))
            if len(set(ns2))>1:
                for (ln,x),n in zip(tbl,ns2): 
                    if n!=ns2[0]: print('   ragged raw',ln,n,x[:100])
            tbl=[]
# identical blocks
def block(lines,start_pat,end_pat):
    s=next(i for i,l in enumerate(lines) if re.match(start_pat,l))
    e=next(i for i,l in enumerate(lines) if i>s and re.match(end_pat,l))
    return lines[s:e]
for name,sp,ep in [('compare',r'^## flowmaid next to mermaid',r'^## '),('perf-next',r'^### Next to mermaid',r'^### Interactive editing'),('large',r'^### Large graphs',r'^## Mermaid parity'),('license',r'^## License',r'^\Z'),]:
    try:
        a=block(nl,sp,ep)
    except StopIteration:
        s=next(i for i,l in enumerate(nl) if re.match(sp,l)); a=nl[s:]
    try:
        b=block(ol,sp,ep)
    except StopIteration:
        s=next(i for i,l in enumerate(ol) if re.match(sp,l)); b=ol[s:]
    print(name,'identical' if a==b else 'DIFFERENT',len(a),len(b))
# versions
for pat in [r'0\.39\.0',r'"0\.39"',r'0\.38\.0',r'113×',r'12\.2\.0',r'12\.2\b',r'v0\.\d+\.\d+',r'MSRV 1\.75',r'mermaid 11',r'mermaid 12\b',r'mermaid v11']:
    print(pat,len(re.findall(pat,new)),len(re.findall(pat,old)))
# numbers in new not in old
numre=re.compile(r'(?<![\w#.])\d[\d,.]*(?:\s?(?:px|ms|%|×|KB|MB|s))?')
on=set(m.group(0).strip().rstrip('.,') for m in numre.finditer(old))
for i,l in enumerate(nl,1):
    for m in numre.finditer(l):
        t=m.group(0).strip().rstrip('.,')
        if t not in on: print('new number',i,repr(t), l[max(0,m.start()-30):m.end()+30])
