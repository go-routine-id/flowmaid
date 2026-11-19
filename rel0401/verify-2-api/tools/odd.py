import re, sys
num = re.compile(r'(?<![\w.#-])-?\d+(?:\.\d+)?(?![\w])')
vbre = re.compile(r'viewBox="([^"]*)"')
b = open(sys.argv[1]).read(); h = open(sys.argv[2]).read()
vbs = vbre.search(h).group(1).split(); ox, oy = float(vbs[0]), float(vbs[1])
h = h.replace('<rect x="%s" y="%s" width=' % (vbs[0], vbs[1]), '<rect width=', 1)
bl = b.split('\n'); hl = h.split('\n')
for i, (x, y) in enumerate(zip(bl, hl)):
    if i == 0: continue
    bn = num.findall(x); hn = num.findall(y)
    odd = []
    for a, c in zip(bn, hn):
        d = float(a) - float(c)
        if abs(d) < 0.111 or abs(d + ox) < 0.151 or abs(d + oy) < 0.151: continue
        odd.append((a, c, round(d, 2)))
    if odd:
        print(i, odd[:8]); print('  B', x[:300]); print('  H', y[:300])
print('ox', ox, 'oy', oy)
