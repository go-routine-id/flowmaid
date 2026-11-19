import sys
a={l.split('\t',1)[0]:l.rstrip('\n') for l in open(sys.argv[1]) if l.strip()}
b={l.split('\t',1)[0]:l.rstrip('\n') for l in open(sys.argv[2]) if l.strip()}
common=[k for k in a if k in b]
same=sum(1 for k in common if a[k]==b[k])
diff=[k for k in common if a[k]!=b[k]]
from collections import Counter
kinds=Counter(a[k].split('\t')[1] for k in common)
cols=Counter()
for k in common: cols[a[k].split('\t')[1]]+=len(a[k].split('\t'))-2
print("compared",len(common),"identical",same,"different",len(diff),"by kind",dict(kinds),"hash columns",sum(cols.values()))
print("panics base",sum(1 for k in common if 'PANIC' in a[k]),"head",sum(1 for k in common if 'PANIC' in b[k]))
for k in diff[:20]:
    print("DIFF",k); print("  base",a[k][:300]); print("  head",b[k][:300])
