import sys, json
for l in open(sys.argv[1]):
    l=l.strip()
    if not l.startswith('{'): print(l[:300]); continue
    r=json.loads(l); i=r['iso']; f=r['full']
    print(f"{r['type']:9} {r['file'][:34]:34} parts={r['parts']:4} iso0 {i['checked0']:8}/{i['bad0']:4} iso4 {i['checked4']:8}/{i['bad4']:4} amb={i['amb']:4} outl={i['outline']:3} | full {f['checked']:7}/{f['bad']:5} amb={f['amb']:4} extra={len(r['extra'])}")
    for s in i['samples'][:8]+f['samples'][:8]+r['extra'][:8]: print('     ', s)
