import re,sys
p=sys.argv[1]
L=open(p,encoding='utf-8').read().split('\n')
incode=False
for i,l in enumerate(L,1):
    if l.startswith('```'): incode=not incode; continue
    if incode or l.startswith('|') or l.startswith('#') or not l.strip(): continue
    t=re.sub(r'\[([^\]]*)\]\([^)]*\)',r'\1',l)
    # split sentences
    sents=re.split(r'(?<=[.!?])\s+(?=[A-Z`\*\[])',t)
    words=len(t.split())
    longs=[(len(s.split()),s) for s in sents if len(s.split())>=34]
    if len(sents)>5 or words>=70 or longs:
        print(f"--- line {i}: {words} words, {len(sents)} sentences")
        for n,s in longs: print(f"     [{n}w] {s[:160]}")
