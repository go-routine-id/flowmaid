import re, subprocess, os
ROOT='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
WT=ROOT+'/wt/rel39'
new=open(WT+'/README.md',encoding='utf-8').read()
old=open(ROOT+'/compare/README.pre-trim-039.md',encoding='utf-8').read()
def spans(t):
    out=[]
    for m in re.finditer(r'(`+)(.+?)\1',t,flags=re.S):
        out.append(m.group(2).strip())
    return out
ns=spans(new); os_=set(spans(old))
src=''
for f in os.listdir(WT+'/src'):
    src+=open(WT+'/src/'+f,encoding='utf-8').read()
ex=''
for f in os.listdir(WT+'/examples'):
    if f.endswith('.rs'): ex+=open(WT+'/examples/'+f,encoding='utf-8').read()
seen=set()
print('--- spans in new not verbatim in old spans:')
for s in ns:
    if s in seen: continue
    seen.add(s)
    if s not in os_:
        print('   ',repr(s), '(substring of old text)' if s in old else '(NOT IN OLD TEXT)')
print('--- rust-ish idents: check existence in src')
idents=set()
for s in seen:
    for m in re.finditer(r'[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_{][A-Za-z0-9_]*)+',s):
        idents.add(m.group(0))
    m=re.fullmatch(r'([a-z_][a-z0-9_]*)\(\)',s)
    if m: idents.add(m.group(1))
for i in sorted(idents):
    last=i.split('::')[-1].split('.')[0]
    if last.startswith('{'): continue
    n=len(re.findall(r'\b(?:fn|struct|enum|const|static|type|mod|trait)\s+'+re.escape(last)+r'\b',src))
    n2=len(re.findall(r'\b'+re.escape(last)+r'\b',src+ex))
    flag='' if n else ('  <-- no def' if not n2 else '  (used, no fn/struct def: field/variant?)')
    print(f'   {i:45s} defs={n} uses={n2}{flag}')
