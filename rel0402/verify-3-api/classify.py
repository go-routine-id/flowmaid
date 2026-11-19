import sys, re, collections
out = collections.defaultdict(list)
for p in open(sys.argv[1]).read().split('\n'):
    if not p: continue
    try: t = open(p, encoding='utf-8', errors='replace').read()
    except Exception: continue
    # strip frontmatter
    if t.lstrip().startswith('---'):
        parts = t.lstrip().split('---', 2)
        t = parts[2] if len(parts) > 2 else ''
    kind = None
    for line in t.split('\n'):
        s = line.strip()
        if not s or s.startswith('%%'): continue
        kind = re.split(r'[\s:;{]', s)[0]
        break
    out[kind or 'none'].append(p)
for k, v in out.items():
    open(sys.argv[2] + '/list_' + re.sub(r'[^A-Za-z0-9-]', '_', k) + '.txt', 'w').write('\n'.join(v) + '\n')
for k, v in sorted(out.items(), key=lambda kv: -len(kv[1]))[:20]:
    print(len(v), k)
