import re,sys
svg=open(sys.argv[1]).read()
print(re.search(r'viewBox="([^"]*)"',svg).group(1))
for m in re.finditer(r'<g><title>([^<]*)</title>\s*<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"',svg):
    print(f"{m.group(1):14s} x={float(m.group(2)):7.1f} y={float(m.group(3)):7.1f}")
for m in re.finditer(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)" rx="8"',svg):
    print('frame',m.groups())
