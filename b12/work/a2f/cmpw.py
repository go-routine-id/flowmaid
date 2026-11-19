import sys
T={}
for ch in "ijl ./\\I'|": T[ch]=3.9
for ch in "!:;ft()[]-`": T[ch]=4.7
for ch in "r*{}": T[ch]=5.5
T['"']=6.7; T['z']=7.0
for ch in "acieksvxyJ0123456789#$_".replace('i',''): T[ch]=7.8
for ch in "+<=>~^": T[ch]=8.2
for ch in "bdghnopqu?FLTZ": T[ch]=8.6
for ch in "EPSVXY": T[ch]=9.4
for ch in "ABCDHKNRU&": T[ch]=10.2
for ch in "wGOQ": T[ch]=10.9
T['M']=11.7; T['m']=12.5; T['%']=12.5; T['W']=13.3; T['@']=13.7
def bw(s): return sum(T.get(c,8.6) for c in s)
def rw(s):
    t=0
    for c in s:
        if c in 'ilj': t+=3.4
        elif c in " .,:;!'tfI|": t+=3.9
        elif c in "r()[]-/": t+=4.7
        elif c in "sckvxyzJ": t+=7.0
        elif c in "mM": t+=11.7
        elif c=='w': t+=10.1
        elif c=='W': t+=13.2
        elif 'A'<=c<='Z': t+=9.7
        else: t+=7.8
    return t
worst=-99; worst_r=-99
for line in open(sys.argv[1]):
    w,s=line.rstrip('\n').split('\t',1)
    w=float(w); e=bw(s)*12/14; r=rw(s)*12/14
    worst=max(worst,w-e); worst_r=max(worst_r,w-r)
    if w-e>0.0 or len(s)>40: print(f"{w:8.2f} bold_est {e:8.2f} ({w-e:+.2f})  reg_est {r:8.2f} ({w-r:+.2f})  {s}")
print("worst under-estimate bold", worst, "regular", worst_r)
