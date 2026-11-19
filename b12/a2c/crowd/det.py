import sys
L=sys.stdin.read().splitlines()
try:
    k=[i for i,l in enumerate(L) if l.strip()=='labels:'][0]
except IndexError:
    print(0, 0.0); sys.exit()
h=L[k+1]
if 'detach' not in h: print(0, 0.0); sys.exit()
i=h.index('detach')
n=0; s=0.0
for l in L[k+2:]:
    try: d=float(l[i-2:i+8].split()[0])
    except: continue
    if d>5: n+=1; s+=d
print(n, round(s,1))
