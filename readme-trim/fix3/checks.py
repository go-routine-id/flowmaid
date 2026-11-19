import re, sys, collections
new_p, old_p = sys.argv[1], sys.argv[2]
new = open(new_p, encoding='utf-8').read()
old = open(old_p, encoding='utf-8').read()
print('bytes', len(old.encode()), '->', len(new.encode()), ' lines', old.count('\n'), '->', new.count('\n'))

# ---------- 1. anchors ----------
def slug(h):
    h = h.strip().lower()
    h = re.sub(r'[^\w\- ]', '', h, flags=re.U)   # drop punctuation (backticks, parens, commas)
    return h.replace(' ', '-')
def headings(text):
    out = []; fence = False
    for l in text.split('\n'):
        if l.startswith('```'): fence = not fence; continue
        if not fence:
            m = re.match(r'(#{1,6}) (.*)', l)
            if m: out.append(m.group(2))
    return out
hs = headings(new)
slugs = collections.Counter(slug(h) for h in hs)
dup = [s for s, c in slugs.items() if c > 1]
links = re.findall(r'\]\(#([^)]+)\)', new)
bad = sorted(set(l for l in links if l not in slugs))
print('1. anchors: %d headings, %d in-page links (%d distinct), unresolved=%s, duplicate slugs=%s' % (len(hs), len(links), len(set(links)), bad, dup))
unlinked = [h for h in hs if slug(h) not in links]
print('   headings with no in-page link:', unlinked)

# ---------- 3. versions, numbers, verbatim blocks ----------
for tok in ['0.39.0', '"0.39"', '0.38.0', '12.2.0', '12.2', 'v0.20.0', '113×']:
    print('3. count %-8s old=%d new=%d' % (tok, old.count(tok), new.count(tok)))
def nums(t):
    t = re.sub(r'```.*?```', '', t, flags=re.S)
    return collections.Counter(re.findall(r'(?<![\w#.])\d[\d,.]*\d|(?<![\w#.])\d', t))
on, nn = nums(old), nums(new)
print('   numbers in new not in old:', sorted(k for k in nn if k not in on))
# verbatim: benchmark tables, comparison images, captions
def tables(t):
    return [l for l in t.split('\n') if l.startswith('|')]
def sect(t, a, b):
    i = t.index(a); j = t.index(b, i); return t[i:j]
for a, b in [('## flowmaid next to mermaid', '## Usage'), ('### Next to mermaid', '### Interactive editing'), ('### Large graphs', '## Mermaid parity roadmap')]:
    print('   verbatim %-28s %s' % (a, sect(old, a, b) == sect(new, a, b)))
imgs_o = re.findall(r'!\[[^\]]*\]\([^)]*\)', old); imgs_n = re.findall(r'!\[[^\]]*\]\([^)]*\)', new)
print('   images identical:', imgs_o == imgs_n, len(imgs_n))
# every performance number of "Interactive editing" kept
ie_o = sect(old, '### Interactive editing', '### Large graphs'); ie_n = sect(new, '### Interactive editing', '### Large graphs')
print('   interactive-editing numbers kept:', sorted(set(nums(ie_o)) - set(nums(ie_n))) == [], sorted(set(nums(ie_n)) - set(nums(ie_o))))

# ---------- 4. backticked names ----------
def ticks(t):
    t = re.sub(r'```.*?```', '', t, flags=re.S) if False else t
    spans = set()
    # fenced blocks removed first
    body = re.sub(r'(?m)^```.*?^```\s*$', '', t, flags=re.S)
    for m in re.finditer(r'(`+)(.+?)\1', body):
        spans.add(m.group(2).strip())
    return spans
ot, nt = ticks(old), ticks(new)
van = sorted(ot - nt)
# a vanished span whose text still appears somewhere in the new README (inside another span or prose)
gone = [v for v in van if v not in new]
print('4. backticked spans: old=%d new=%d vanished=%d (of which text gone entirely=%d), new-only=%d' % (len(ot), len(nt), len(van), len(gone), len(nt - ot)))
print('   NEW-ONLY spans:', sorted(nt - ot))
import json
json.dump({'vanished': van, 'gone': gone}, open(sys.argv[3], 'w'), ensure_ascii=False, indent=1)
