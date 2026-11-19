import sys, re
def fn_body(path, name):
    src = open(path).read()
    m = re.search(r'\n(pub(\([a-z]+\))? )?fn ' + re.escape(name) + r'\b', src)
    if not m: return None
    i = src.index('{', m.end())
    depth = 0
    for j in range(i, len(src)):
        if src[j] == '{': depth += 1
        elif src[j] == '}':
            depth -= 1
            if depth == 0:
                return src[m.start():j+1]
    return None
a, b = sys.argv[1], sys.argv[2]
for name in sys.argv[3:]:
    x, y = fn_body(a, name), fn_body(b, name)
    print("%-22s %s" % (name, "MISSING" if x is None or y is None else ("identical" if x == y else "DIFFERENT")))
