import hashlib, subprocess, sys, os, collections
S = sys.argv[1]; NEW = sys.argv[2]; OLD = sys.argv[3]; roots = sys.argv[4:]
seen = {}
for root in roots:
    for dp, dn, fn in os.walk(root):
        if '/target' in dp or '/verify/b0-metrics/pre' in dp or '/.git' in dp:
            continue
        for f in fn:
            if f.endswith('.mmd'):
                p = os.path.join(dp, f)
                try:
                    h = hashlib.md5(open(p, 'rb').read()).hexdigest()
                except Exception:
                    continue
                seen.setdefault(h, p)
kinds = collections.Counter(); diffs = []; errs = 0; n = 0
for h, p in sorted(seen.items(), key=lambda x: x[1]):
    try:
        a = subprocess.run([NEW, p], capture_output=True, timeout=30)
        b = subprocess.run([OLD, p], capture_output=True, timeout=30)
    except subprocess.TimeoutExpired:
        print('TIMEOUT', p); continue
    n += 1
    first = open(p, errors='replace').read().lstrip()
    head = ''
    for line in first.splitlines():
        line = line.strip()
        if line and not line.startswith('%%') and not line.startswith('---'):
            head = line.split()[0]; break
    kinds[head] += 1
    if a.returncode != b.returncode or a.stdout != b.stdout:
        diffs.append(p)
        print('DIFF', p, a.returncode, b.returncode)
    if a.returncode != 0:
        errs += 1
print('files', n, 'diffs', len(diffs), 'nonzero-exit', errs)
print(dict(kinds.most_common(25)))
