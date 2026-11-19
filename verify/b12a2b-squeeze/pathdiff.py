import re,sys
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a2b-squeeze'
for n in sys.argv[1:]:
    a=open(f'{V}/son-pre/{n}').read(); b=open(f'{V}/son-head/{n}').read()
    pa=re.findall(r'<path[^>]*>',a); pb=re.findall(r'<path[^>]*>',b)
    print('==',n, len(pa), len(pb))
    for x,y in zip(pa,pb):
        if x!=y:
            ida=re.search(r'(?:id|data-edge|class)="([^"]*)"',x)
            print(' PRE ', x[:600]); print(' HEAD', y[:600])
    ma=re.findall(r'<mask .*?</mask>',a); mb=re.findall(r'<mask .*?</mask>',b)
    for x,y in zip(ma,mb):
        if x!=y: print(' MASK PRE ',x[:400]); print(' MASK HEAD',y[:400])
    if len(ma)!=len(mb): print(' masks',len(ma),len(mb))
