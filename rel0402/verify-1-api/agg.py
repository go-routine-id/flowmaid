import sys,re,collections,glob
agg=collections.defaultdict(lambda:[0,0]); samples=collections.defaultdict(list); files=collections.Counter(); exits=collections.Counter()
for f in sorted(glob.glob(sys.argv[1]+'/*.out')):
    for l in open(f):
        m=re.match(r'^(\S+)\s+(\d+) fail (\d+)$', l.strip())
        if m: agg[m[1]][0]+=int(m[2]); agg[m[1]][1]+=int(m[3]); continue
        m=re.match(r'^files: \{(.*)\}', l.strip())
        if m:
            for k,v in re.findall(r'"(\w+)": (\d+)', m[1]): files[k]+=int(v)
            continue
        if l.startswith('exit'): exits[l.strip()]+=1; continue
        if l.startswith('  '):
            k=l.strip().split(':')[0]
            if len(samples[k])<4: samples[k].append(l.strip()[:230])
print('files', dict(files), 'exits', dict(exits))
for k,(n,f) in sorted(agg.items()):
    print(f'{k:45} {n:8} {"FAIL "+str(f) if f else "ok"}')
print('total checks', sum(n for n,f in agg.values()))
for k,v in samples.items():
    for s in v: print('   ', s)
