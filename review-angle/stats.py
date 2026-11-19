import re,sys
p=sys.argv[1]
L=open(p,encoding='utf-8').read().split('\n')
# bytes per H2/H3 section
secs=[];cur=['(top)',0,1,0]
incode=False
for i,l in enumerate(L,1):
    if l.startswith('```'): incode=not incode
    if not incode and re.match(r'^#{1,3} ',l):
        secs.append(cur);cur=[l,0,i,0]
    cur[1]+=len(l.encode())+1;cur[3]+=1
secs.append(cur)
for s in secs: print(f"{s[1]:6d} B {s[3]:4d} L  @{s[2]:<4d} {s[0]}")
