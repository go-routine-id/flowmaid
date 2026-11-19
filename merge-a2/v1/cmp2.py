import sys, glob
def load(fs):
    d={}
    for f in fs:
        for l in open(f, encoding='utf-8', errors='replace'):
            l=l.rstrip('\n')
            if l: d[l.split('\t',1)[0]]=l
    return d
a=load(sorted(glob.glob(sys.argv[1]))); b=load(sorted(glob.glob(sys.argv[2])))
c=sorted(set(a)&set(b))
diff=[p for p in c if a[p]!=b[p] and 'EXIT=' not in a[p]+b[p]]
ex=[p for p in c if 'EXIT=' in a[p]+b[p]]
print(f"common {len(c)}; compact rows {sum(1 for p in c if 'compact600=' in a[p])}; differences {len(diff)}; exits {len(ex)}")
for p in diff[:10]:
    print("DIFF", p, [x for x,y in zip(a[p].split('\t'),b[p].split('\t')) if x!=y][:4])
for p in ex: print("EXIT", p)
