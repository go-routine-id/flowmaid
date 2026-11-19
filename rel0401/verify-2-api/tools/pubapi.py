import re, sys, os
root = sys.argv[1]
out = []
for dp, dn, fns in os.walk(os.path.join(root, 'src')):
    for fn in sorted(fns):
        if not fn.endswith('.rs'): continue
        p = os.path.join(dp, fn); rel = os.path.relpath(p, root)
        lines = open(p, encoding='utf8').read().split('\n')
        # stop at #[cfg(test)] mod tests
        i = 0; n = len(lines)
        attrs = []
        while i < n:
            l = lines[i]
            if re.match(r'\s*#\[cfg\(test\)\]', l):
                # skip following item block crudely: if next is mod tests, break
                if i+1 < n and re.match(r'\s*mod\s+\w+', lines[i+1]): break
            s = l.strip()
            if s.startswith('#[') and not s.startswith('#[doc'):
                attrs.append(s)
            if re.match(r'\s*pub(\s|$)', l) and not re.match(r'\s*pub\(', l):
                sig = s
                j = i
                while not re.search(r'[{;]\s*$', sig) and not re.search(r',\s*$', sig) and j+1 < n and j - i < 30:
                    j += 1; sig += ' ' + lines[j].strip()
                sig = re.sub(r'\s+', ' ', sig)
                sig = re.sub(r'\s*\{.*$', '', sig)
                a = ' '.join(x for x in attrs if 'non_exhaustive' in x or 'derive' in x)
                out.append(f"{rel}: {a+' ' if a else ''}{sig}")
                i = j
            if s and not s.startswith('#[') and not s.startswith('///') and not s.startswith('//'):
                attrs = []
            i += 1
print('\n'.join(out))
