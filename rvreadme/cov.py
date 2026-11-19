import re,sys
old=open(sys.argv[1]).read(); new=open(sys.argv[2]).read()
newn=re.sub(r'\s+',' ',new)
# split old into clauses on ; . — 
lines=old.split('\n')
for i,l in enumerate(lines,1):
    if l.startswith('|') or l.startswith('!['): continue
    for cl in re.split(r'(?<=[.;:])\s+|\s—\s|, and |\(|\)',l):
        cl=cl.strip()
        if len(cl)<25: continue
        # check n-gram presence: 4-word shingles
        w=cl.split()
        if len(w)<4: continue
        sh=[' '.join(w[j:j+4]) for j in range(len(w)-3)]
        hit=sum(1 for s in sh if s in newn)
        if 0.5<=hit/len(sh)<0.95 and len(sh)-hit>=3:
            print(f"{i}: [{hit}/{len(sh)}] {cl}")
