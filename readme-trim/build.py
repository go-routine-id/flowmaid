import re, sys
old = open(sys.argv[1], encoding='utf-8').read().split('\n')
tmpl = open(sys.argv[2], encoding='utf-8').read().split('\n')
out = []
for line in tmpl:
    m = re.fullmatch(r'@@OLD:(\d+)-(\d+)@@', line)
    if m:
        a, b = int(m.group(1)), int(m.group(2))
        out.extend(old[a-1:b])
    else:
        out.append(line)
text = '\n'.join(out)
if not text.endswith('\n'):
    text += '\n'
open(sys.argv[3], 'w', encoding='utf-8').write(text)
