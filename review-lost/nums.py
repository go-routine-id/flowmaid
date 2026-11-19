import re,sys
def body(p):
    s=open(p,encoding='utf-8').read()
    return s
o=body(sys.argv[1]); n=body(sys.argv[2])
pat=re.compile(r'(?<![\w#.])\d[\d,]*(?:\.\d+)?(?:\s?(?:px|ms|KB|MB|%|×|s\b|:1| : 1))?')
from collections import Counter
co=Counter(m.group(0).strip() for m in pat.finditer(o)); cn=Counter(m.group(0).strip() for m in pat.finditer(n))
print('numbers in old missing from new:')
for k in sorted(co):
    if k not in cn: print('  ',repr(k),co[k])
print('numbers new not in old:')
for k in sorted(cn):
    if k not in co: print('  ',repr(k),cn[k])
