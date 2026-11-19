import re, sys, collections
NEW='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/wt/rel39/README.md'
OLD='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/compare/README.pre-trim-039.md'
new=open(NEW,encoding='utf-8').read()
old=open(OLD,encoding='utf-8').read()
nl=new.split('\n'); ol=old.split('\n')

def slug(h):
    s=h.strip().lower()
    s=re.sub(r'`','',s)
    s=re.sub(r'[^\w\- ]','',s,flags=re.U)
    s=s.replace(' ','-')
    return s
def headings(lines):
    out=[]; fence=False
    for i,l in enumerate(lines,1):
        if l.startswith('```'): fence=not fence; continue
        if fence: continue
        m=re.match(r'^(#{1,6})\s+(.*)$',l)
        if m: out.append((i,len(m.group(1)),m.group(2)))
    return out
H=headings(nl)
slugs=collections.Counter()
sl=[]
for i,lv,t in H:
    s=slug(t); n=slugs[s]; slugs[s]+=1
    sl.append((i,lv,t,s if n==0 else f'{s}-{n}'))
S={s for *_,s in sl}
print('headings',len(H))
for x in sl: print('  ',x)
dups=[s for s,c in slugs.items() if c>1]
print('dup slugs',dups)
# links
fence=False; links=[]
for i,l in enumerate(nl,1):
    if l.startswith('```'): fence=not fence; continue
    if fence: continue
    for m in re.finditer(r'\]\((#[^)]+)\)',l): links.append((i,m.group(1)[1:]))
print('in-page links',len(links),'distinct',len({a for _,a in links}))
for i,a in links:
    if a not in S: print('  UNRESOLVED',i,a)
# headings not linked from contents (level 2)
contents=[l for l in nl[46:56]]
ctext='\n'.join(contents)
for i,lv,t,s in sl:
    if lv==2 and ('#'+s+')') not in ctext and t!='Contents': print('  H2 not in contents:',t)
# external links / relative paths
for i,l in enumerate(nl,1):
    for m in re.finditer(r'\]\(([^)#][^)]*)\)',l):
        u=m.group(1)
        if not u.startswith('http'): print('  rel link',i,u)
