import re,sys
def toks(p):
    s=open(p,encoding='utf-8').read()
    # strip fenced blocks separately
    fenced=re.findall(r'```.*?```',s,flags=re.S)
    body=re.sub(r'```.*?```','',s,flags=re.S)
    t=set(re.findall(r'``\s?(.+?)\s?``',body))
    body2=re.sub(r'``\s?(.+?)\s?``','',body)
    t|=set(re.findall(r'`([^`\n]+)`',body2))
    return t,fenced,s
old,of,os_=toks(sys.argv[1]); new,nf,ns=toks(sys.argv[2])
gone=sorted(old-new)
print(len(old),len(new),len(gone))
for g in gone:
    # is it still present as substring in new text?
    sub = g in ns
    print(('  [substr-present] ' if sub else '  [GONE] ')+g)
print('--- added')
for a in sorted(new-old): print('  +',a)
