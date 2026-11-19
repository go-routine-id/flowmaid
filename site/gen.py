import json, html, re
S='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
dims=json.load(open(f'{S}/site/dims.json'))
per={r['name']:r for r in json.load(open(f'{S}/perdiagram.json'))}
def md(s):
    s=html.escape(s.strip())
    s=re.sub(r'`([^`]+)`', r'<code>\1</code>', s)
    s=re.sub(r'\*\*([^*]+)\*\*', r'<strong>\1</strong>', s)
    return s
rows={}
for line in open(f'{S}/site/table.md').read().splitlines()[2:]:
    c=[x for x in line.strip().strip('|').split(' | ')]
    rows[c[0].strip()]=c[1:]
FLOW=["demo","advanced","subgraph","lr","shapes","stress","fanout","long-labels","back-edge","wide-labels","fanout-frames","dense-td","dense-lr","bench-75"]
OTHER=["class","er","state","sequence","pie","mindmap","journey","gitgraph","timeline","sankey","architecture"]
def img(key, alt):
    w,h=dims[key]
    return f'<img src="img/{key}.webp" width="{w}" height="{h}" alt="{html.escape(alt)}" loading="lazy">'
cards=[]
for n in FLOW+OTHER:
    verdict, issues, better = rows[n]
    v=[i for i in per[n]['verified'] if i['verdict']!='refuted']
    heavy=sum(1 for i in v if i['severity']==3)
    grp='flow' if n in FLOW else 'other'
    iss=''.join(f'<li>{md(x)}</li>' for x in issues.split(';') if x.strip())
    sev=f'<span class="chip {"chip-hot" if heavy>=3 else "chip-warm" if heavy else "chip-ok"}">{heavy} berat · {len(v)} total</span>'
    cards.append(f'''<article class="card" data-group="{grp}" id="d-{n}">
<header class="card-head"><h3><code>{n}.mmd</code></h3>{sev}</header>
<p class="verdict">{md(verdict)}</p>
<div class="pair">
<figure><figcaption>mermaid.js 12.2.0</figcaption><div class="canvas">{img(n+".mermaid", n+" dirender mermaid")}</div></figure>
<figure><figcaption>flowmaid 0.32</figcaption><div class="canvas">{img(n+".flowmaid", n+" dirender flowmaid")}</div></figure>
</div>
<div class="notes"><div><h4>Isu teratas</h4><ul>{iss}</ul></div><div><h4>Flowmaid sudah lebih baik</h4><p>{md(better)}</p></div></div>
</article>''')
tpl=open(f'{S}/site/template.html').read()
open(f'{S}/site/index.html','w').write(tpl.replace('<!--CARDS-->','\n'.join(cards)).replace('<!--PROTO-DEMO-BEFORE-->',img('demo.flowmaid','demo sekarang')).replace('<!--PROTO-DEMO-AFTER-->',img('proto-demo','demo dengan lane back-edge')).replace('<!--PROTO-ADV-BEFORE-->',img('advanced.flowmaid','advanced sekarang')).replace('<!--PROTO-ADV-AFTER-->',img('proto-advanced','advanced dengan fix subgraph')).replace('<!--PROTO-LR-->',img('proto-lr','lr sebelum dan sesudah')))
print('ok')
