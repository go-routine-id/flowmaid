import sys
L=sys.stdin.read().splitlines()
k=[i for i,l in enumerate(L) if l.strip()=='labels:'][0]
h=L[k+1]; i=h.index('detach'); t=h.index(' text'); x=h.index('     x ') if '     x ' in h else None
for l in L[k+2:]:
    try: d=float(l[i-2:i+8].split()[0])
    except: continue
    if d>5: print(' ', l[:t].strip(), '|', l[t:t+30].strip(), '| detach', d)
