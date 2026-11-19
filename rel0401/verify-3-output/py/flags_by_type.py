import sys,collections
def kind(f):
    try: t=open(f,encoding='utf-8',errors='replace').read()
    except: return '?'
    for l in t.split('\n'):
        s=l.strip()
        if not s or s.startswith('%%') or s=='---': continue
        return s.split()[0]
    return '?'
cache={}
c=collections.Counter(); ex={}
for p in sys.argv[1:]:
    for l in open(p):
        r=l.rstrip('\n').split('\t')
        if len(r)<4: continue
        for t in r[3].split(';'):
            if t[:1].isupper():
                f=r[0]
                if f not in cache: cache[f]=kind(f)
                key=(cache[f], t.split('=')[0].split(' ')[0]); c[key]+=1; ex.setdefault(key,(f,r[1],r[3][:200]))
for k,v in sorted(c.items()): print(v,k,ex[k])
