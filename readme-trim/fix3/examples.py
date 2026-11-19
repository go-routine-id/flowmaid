import re, sys, subprocess, os, json
readme, old_p, fm, out, repo = sys.argv[1:6]
text = open(readme, encoding='utf-8').read()
old = open(old_p, encoding='utf-8').read()
blocks = re.findall(r'(?m)^```(\w*)\n(.*?)^```\s*$', text, flags=re.S)
print('fenced blocks:', [(l, len(b.split('\n')) - 1) for l, b in blocks])
print('every fenced block is verbatim in the old README:', all(b in old for _, b in blocks) or [l for l, b in blocks if b not in old])
res = []
def run(name, args, stdin=None, expect_ok=True):
    p = subprocess.run([fm] + args, input=stdin, capture_output=True, cwd=repo)
    ok = (p.returncode == 0)
    svg = None
    for a in args:
        if a.endswith('.svg') and os.path.exists(a): svg = open(a, encoding='utf-8').read()
    if svg is None and p.stdout: svg = p.stdout.decode('utf-8', 'replace')
    good = ok and svg is not None and svg.lstrip().startswith('<svg') and svg.rstrip().endswith('</svg>')
    res.append((name, good if expect_ok else (not ok), p.returncode, (p.stderr.decode()[:90].strip() if not ok else '')))
    return svg
n = 0
for lang, body in blocks:
    n += 1
    if lang == 'mmd':
        f = os.path.join(out, 'block%d.mmd' % n); open(f, 'w').write(body)
        run('fenced mmd #%d (--advance-text)' % n, ['--advance-text', f, '-o', f[:-4] + '.svg'])
    elif lang == 'json':
        f = os.path.join(out, 'block%d.json' % n); open(f, 'w').write(body)
        run('fenced json #%d (--advance)' % n, ['--advance', f, '-o', f[:-5] + '.svg'])
    elif lang == 'rust':
        for k, m in enumerate(re.finditer(r'render_svg(?:_advanced)?\("((?:[^"\\]|\\.)*)"', body)):
            src = m.group(1).encode().decode('unicode_escape')
            f = os.path.join(out, 'rust%d_%d.mmd' % (n, k)); open(f, 'w').write(src + '\n')
            run('rust string #%d.%d %r' % (n, k, src), [f, '-o', f[:-4] + '.svg'])
# CLI lines of the bash blocks and the options table
o = lambda x: os.path.join(out, x)
run('cli: flowmaid examples/demo.mmd -o demo.svg', ['examples/demo.mmd', '-o', o('demo.svg')])
run('cli: cat examples/lr.mmd | flowmaid > lr.svg', [], stdin=open(os.path.join(repo, 'examples/lr.mmd'), 'rb').read())
run('cli: --compact 600', ['examples/shapes.mmd', '--compact', '600', '-o', o('compact.svg')])
run('cli: --transparent', ['examples/demo.mmd', '--transparent', '-o', o('transparent.svg')])
for th in ['flowmaid', 'default', 'neutral', 'dark', 'forest']:
    run('cli: --theme ' + th, ['examples/demo.mmd', '--theme', th, '-o', o('theme-%s.svg' % th)])
run('cli: --advance-text examples/advance_swimlane.mmd', ['--advance-text', 'examples/advance_swimlane.mmd', '-o', o('adv1.svg')])
run('cli: --advance-text examples/advance_terminals.mmd', ['--advance-text', 'examples/advance_terminals.mmd', '-o', o('adv2.svg')])
p = subprocess.run([fm, '--help'], capture_output=True); res.append(('cli: --help', p.returncode == 0, p.returncode, ''))
# every example file the README names
for ex in sorted(set(re.findall(r'`([\w-]+\.mmd)`', text))):
    path = os.path.join(repo, 'examples', ex)
    if not os.path.exists(path): res.append(('example file ' + ex, False, -1, 'missing')); continue
    if ex.startswith('advance'): continue
    run('example file ' + ex, ['examples/' + ex, '-o', o('ex-' + ex[:-4] + '.svg')])
for ex in sorted(set(re.findall(r'`examples/([\w./-]+)`', text))):
    res.append(('named path examples/' + ex, os.path.exists(os.path.join(repo, 'examples', ex)), 0, ''))
# inline examples named in the prose (bullets and tables)
inline = {
 'metadata + edge id + animate': 'flowchart TD\nA@{ shape: cyl, label: "DB" }\nA e1@--> B\ne1@{ animate: true }\n',
 'asymmetric A>text] + curve + interpolate': 'flowchart TD\nA>text] e1@--> B\nB --> C\ne1@{ curve: natural }\nlinkStyle 1 interpolate basis\n',
 'direction holds (X --> s, s --> Y)': 'flowchart TB\nsubgraph s\ndirection LR\nA --> B --> C\nend\nX --> s\ns --> Y\n',
 'direction lost (C --> s)': 'flowchart TB\nsubgraph s\ndirection LR\nA --> B --> C\nC --> s\nend\n',
 'member to its own box (Dev --> CI)': 'flowchart TD\nsubgraph CI\nDev --> Build\nend\nDev --> CI\n',
 'empty markdown label': 'flowchart TD\nA["``"] --> B\n',
 'click with extra spaces': 'flowchart TD\nA --> B\nclick A  href  "u"\n',
 'click callbacks': 'flowchart TD\nA --> B\nclick A callback\nclick B call fn(args)\n',
 'entity codes #0; #27;': 'flowchart TD\nA["a#0;b#27;c"]\n',
 'math': 'flowchart TD\nA["$$x^2$$"] --> B\n',
 'state note as N': 'stateDiagram-v2\n[*] --> A\nnote "text" as N\n',
 'ER alias in a relationship + pk, fk': 'erDiagram\np[Person] ||--o{ c[Car] : owns\nCar { int id pk, fk }\n',
 'sequence scopedFrames + mirrorActors': '---\nconfig:\n  sequence:\n    scopedFrames: true\n    mirrorActors: false\n---\nsequenceDiagram\n    participant A\n    participant B\n    A->>B: hi\n    loop retry\n        B->>B: check\n    end\n',
 'gitGraph TB:': 'gitGraph TB:\n    commit id: "a"\n    branch f order: 2\n    switch f\n    commit type: REVERSE msg: "m" tag: "t"\n    checkout main\n    merge f\n',
 'mindmap icon + classes + classDef': 'mindmap\n  root((flowmaid))\n    Urgent\n    :::urgent large\n    ::icon(fa fa-book)\n    Other\n  classDef urgent fill:#ffe066,stroke:#c92a2a,color:#222\n',
 'timeline bare ": event" + url colon': 'timeline\n    title T\n    2002 : LinkedIn : https://example.com\n         : Google\n',
 'sankey spelling + quoted field': 'sankey\n"a, ""b""",c,3\nc,d,2\n',
 'architecture port edges + {group}': 'architecture-beta\n    group g(cloud)[G]\n    service a(server)[A] in g\n    service b(database)[B]\n    junction j\n    a:R -[label]- L:b\n    a{group}:B --> T:j\n',
 'pie showData + title line': 'pie showData\n    title T\n    "A" : 40\n    "B" : 60\n',
 'journey fractional score': 'journey\n    title J\n    section S\n        Task: 3.5: Me, Cat\n',
}
for k, src in inline.items():
    f = o('inline-' + re.sub(r'\W+', '_', k)[:40] + '.mmd'); open(f, 'w').write(src)
    run('inline: ' + k, [f, '-o', f[:-4] + '.svg'])
# documented errors
errs = {
 'unsupported header gantt -> "not supported yet"': ('gantt\n  title x\n', 'not supported yet'),
 'sequence <<->> -> "unknown message operator"': ('sequenceDiagram\nA<<->>B: x\n', 'unknown message operator'),
 'sequence box -> "not supported yet"': ('sequenceDiagram\nbox Blue\nparticipant A\nend\n', 'not supported yet'),
 'unknown shape -> "No such shape"': ('flowchart TD\nA@{ shape: Nope_x }\n', 'No such shape'),
 'venn-beta -> "unknown edge operator"': ('venn-beta\n  set A\n', 'unknown edge operator'),
 'class dotted name -> parse error': ('classDiagram\nA.B --> C\n', 'error'),
 'raw quote in markdown string -> line-numbered error': ('flowchart TD\nA["`a "b" c`"]\n', 'line'),
}
for k, (src, want) in errs.items():
    f = o('err-' + re.sub(r'\W+', '_', k)[:40] + '.mmd'); open(f, 'w').write(src)
    p = subprocess.run([fm, f, '-o', f[:-4] + '.svg'], capture_output=True)
    msg = (p.stderr + p.stdout).decode('utf-8', 'replace')
    res.append(('error: ' + k, p.returncode != 0 and want in msg, p.returncode, msg.strip()[:100]))
bad = [r for r in res if not r[1]]
for r in res: print(('ok  ' if r[1] else 'FAIL'), r[0], '' if r[1] else r[2:])
print('TOTAL', len(res), 'failed', len(bad))
