# Build the updated "Flowmaid vs Mermaid" page from the review results.
# usage: python3 -I gen.py <fvm dir>   (reads old/index.html, old-cards.json, review.json, img/*.webp; writes page/index.html)
import sys, os, re, json, html
from PIL import Image

F = sys.argv[1]
old = open(os.path.join(F, 'old/index.html')).read()
cards = json.load(open(os.path.join(F, 'old-cards.json')))
rev = json.load(open(os.path.join(F, 'review.json')))
diag = {d['name']: d for d in rev['diagrams']}
status = rev['status']
E = lambda s: html.escape(str(s), quote=True)

# old image sizes per card (mermaid + 0.32)
oldsz = {}
for m in re.finditer(r'<img src="img/([\w-]+)\.(mermaid|flowmaid)\.webp" width="(\d+)" height="(\d+)"', old):
    oldsz[(m.group(1), m.group(2))] = (int(m.group(3)), int(m.group(4)))

def wh(name):
    im = Image.open(os.path.join(F, 'img', name))
    return im.size

# old reasons (title + body) and bug rows
reasons = []
for m in re.finditer(r'<li><strong>(.*?)</strong><span>(.*?)</span></li>', re.search(r'<ol class="reasons">(.*?)</ol>', old, re.S).group(1), re.S):
    reasons.append((m.group(1), m.group(2)))
bugs = []
for m in re.finditer(r'<tr><td class="n">(\d+)</td><td>(.*?)</td><td>(.*?)</td><td>(.*?)</td><td>(.*?)</td></tr>', old):
    bugs.append(dict(n=int(m.group(1)), bug=m.group(2), diagram=m.group(3), cause=m.group(4)))

ST_CHIP = {'beres': ('chip-ok', 'beres'), 'sebagian': ('chip-warm', 'sebagian'), 'masih': ('chip-hot', 'masih ada'), 'belum': ('chip-hot', 'belum')}
V_CHIP = {'lebih baik': 'chip-ok', 'setara': 'chip-acc', 'campur': 'chip-warm', 'masih kalah': 'chip-hot'}

def chip(status_key):
    cls, txt = ST_CHIP[status_key]
    return f'<span class="chip {cls}">{txt}</span>'

# counts
vc = {k: 0 for k in V_CHIP}
for d in rev['diagrams']:
    vc[d['verdict']] += 1
oi = {'beres': 0, 'sebagian': 0, 'masih': 0}
for d in rev['diagrams']:
    for o in d['old_issues']:
        oi[o['status']] += 1
n_old = sum(oi.values())
rs = {s['n']: s for s in status['reasons']}
bs = {s['n']: s for s in status['bugs']}
rb = sum(1 for s in status['reasons'] if s['status'] == 'beres')
bb = sum(1 for s in status['bugs'] if s['status'] == 'beres')

out = []
w = out.append
w(open(os.path.join(F, 'tools/head.html')).read())

# hero
w(f'''<div class="wrap">
<header class="hero">
  <div class="eyebrow">LAPORAN PARITAS VISUAL · DIPERBARUI 19 NOV 2026</div>
  <h1>Flowmaid vs Mermaid</h1>
  <p class="lede col">Sumber <code>.mmd</code> yang sama, dirender dengan mermaid.js 12.2.0 dan flowmaid 0.40.2. Laporan pertama (30 Oktober, flowmaid 0.32) menyimpulkan bahwa flowmaid kalah di <strong>komposisi</strong>: bentuk edge, back-edge, kepala panah, dan layout cluster. Sepuluh rilis kemudian, halaman ini memeriksa ulang setiap temuan itu pada gambar hari ini.</p>
  <div class="meta"><span><b>25</b> diagram</span><span>lebih baik <b>{vc['lebih baik']}</b> · setara <b>{vc['setara']}</b> · campur <b>{vc['campur']}</b> · masih kalah <b>{vc['masih kalah']}</b></span><span>isu 30 Okt: <b>{oi['beres']}</b> beres · <b>{oi['sebagian']}</b> sebagian · <b>{oi['masih']}</b> masih ada (dari {n_old})</span><span>commit <b>c202cfc</b> (0.40.2)</span></div>
</header>
''')

# reasons
w('<section class="col">\n  <h2>Lima alasan terbesar, sekarang</h2>\n  <p class="sub">Lima penyebab utama dari laporan 30 Oktober. Statusnya diperiksa di CHANGELOG dan di gambar 0.40.2.</p>\n  <ol class="reasons">\n')
for i, (title, body) in enumerate(reasons, 1):
    s = rs[i]
    rel = '' if s['release'] in ('-', '') else f'<span class="rel">{E(s["release"])}</span>'
    w(f'    <li><div class="rtop"><strong>{title}</strong>{chip(s["status"])}{rel}</div><span class="now">{E(s["note"])}</span><details><summary>Temuan 30 Okt</summary><span>{body}</span></details></li>\n')
w('  </ol>\n</section>\n')

# bugs table
w(f'''<section>
  <div class="col"><h2>Bug kebenaran</h2><p class="sub">Sepuluh bug yang membuat gambar salah atau kehilangan informasi. {bb} dari 10 sudah beres.</p></div>
  <div class="tablewrap"><table>
    <thead><tr><th>#</th><th>Bug (30 Okt)</th><th>Diagram</th><th>Status</th><th>Sekarang</th></tr></thead>
    <tbody>
''')
for b in bugs:
    s = bs[b['n']]
    rel = '' if s['release'] in ('-', '') else f'<div class="rel">{E(s["release"])}</div>'
    w(f'      <tr><td class="n">{b["n"]}</td><td>{b["bug"]}</td><td>{b["diagram"]}</td><td>{chip(s["status"])}{rel}</td><td>{E(s["note"])}</td></tr>\n')
w('    </tbody>\n  </table></div>\n</section>\n')

# blocks section
bm, bd, bk = wh('blocks-overview.mermaid.webp'), wh('blocks-overview.default.webp'), wh('blocks-overview.blocks.webp')
w(f'''<section>
  <div class="col">
    <h2>Sedang dikerjakan: subgraph sebagai blok</h2>
    <p>Diagram dengan banyak subgraph yang saling terhubung masih kalah rapi dari mermaid. Layout baru membuat setiap subgraph jadi satu blok padat: blok yang terhubung ditumpuk searah alur, edge berjalan di celah antar-blok, dan label punya ruang sendiri. Layout ini sudah ada di kode, tetapi masih dimatikan sampai lolos perbandingan dengan mermaid.</p>
  </div>
  <div class="three">
    <figure><figcaption>mermaid.js 12.2.0</figcaption><div class="canvas"><img src="img/blocks-overview.mermaid.webp" width="{bm[0]}" height="{bm[1]}" alt="blocks-overview dirender mermaid" loading="lazy"></div></figure>
    <figure><figcaption>flowmaid 0.40.2 (sekarang)</figcaption><div class="canvas"><img src="img/blocks-overview.default.webp" width="{bd[0]}" height="{bd[1]}" alt="blocks-overview dirender flowmaid 0.40.2" loading="lazy"></div></figure>
    <figure><figcaption>mode blok (belum dirilis)</figcaption><div class="canvas"><img src="img/blocks-overview.blocks.webp" width="{bk[0]}" height="{bk[1]}" alt="blocks-overview dalam mode blok" loading="lazy"></div></figure>
  </div>
  <div class="col">
    <p class="note">Input: <code>tests/fixtures/blocks/blocks-overview.mmd</code>. Gambar mode blok diambil dari branch kerja (commit b82d584), tanpa drag.</p>
    <div class="tablewrap steps"><table>
      <thead><tr><th>Langkah</th><th>Isi</th><th>Status</th></tr></thead>
      <tbody>
        <tr><td class="n">1</td><td>Struktur: pohon blok, layering, urutan</td><td><span class="chip chip-ok">masuk 0.40.1</span> dimatikan</td></tr>
        <tr><td class="n">2</td><td>Geometri: frame, koordinat, ruang untuk celah dan label. Bagian kedua (frame lebih rapat, rantai tetap lurus) belum mencapai target setelah dua putaran kerja.</td><td><span class="chip chip-warm">sebagian</span> bagian pertama masuk 0.40.2</td></tr>
        <tr><td class="n">3</td><td>Routing saat drag: <code>route()</code> menggambar sama dengan layout statis, drag hanya menggambar ulang yang bergerak.</td><td><span class="chip chip-ok">lolos review</span> di branch kerja</td></tr>
        <tr><td class="n">4</td><td>Koridor untuk edge yang digambar ulang, perbandingan penuh dengan mermaid, lalu jadi default</td><td><span class="chip chip-hot">belum mulai</span></td></tr>
      </tbody>
    </table></div>
    <p class="note">Diagram uji padat (<code>dense-td</code>): mode blok menggambar 42 crossing, layout sekarang 66 (statis) dan 76 (setelah <code>route()</code>).</p>
  </div>
</section>
''')

# gallery
nflow = sum(1 for c in cards if c['group'] == 'flow'); noth = len(cards) - nflow
w(f'''<section>
  <div class="col"><h2>Galeri per diagram</h2><p class="sub">Mermaid di kiri, flowmaid 0.40.2 di kanan, dari input yang identik. Chip menunjukkan penilaian 0.40.2 terhadap mermaid. Di bawahnya, isu dari laporan 30 Oktober dengan statusnya sekarang. Gambar 0.32 ada di bagian yang bisa dibuka.</p></div>
  <div class="filters" role="group" aria-label="Saring diagram">
    <button type="button" id="f-all" data-f="all" aria-pressed="true">Semua ({len(cards)})</button>
    <button type="button" id="f-flow" data-f="flow" aria-pressed="false">Flowchart ({nflow})</button>
    <button type="button" id="f-other" data-f="other" aria-pressed="false">Tipe lain ({noth})</button>
    <button type="button" id="f-behind" data-f="behind" aria-pressed="false">Belum setara ({vc['campur'] + vc['masih kalah']})</button>
  </div>
  <nav class="toc" aria-label="Lompat ke diagram" id="toc"></nav>
  <div class="cards" id="cards">
''')
for c in cards:
    n = c['name']; d = diag[n]
    mw, mh = oldsz[(n, 'mermaid')]; ow, oh = oldsz[(n, 'flowmaid')]
    nw, nh = wh(f'{n}.fm0402.webp')
    behind = 'yes' if d['verdict'] in ('campur', 'masih kalah') else 'no'
    w(f'<article class="card" data-group="{c["group"]}" data-behind="{behind}" id="d-{n}">\n')
    w(f'<header class="card-head"><h3><code>{n}.mmd</code></h3><span class="chip {V_CHIP[d["verdict"]]}">{E(d["verdict"])}</span></header>\n')
    w(f'<p class="verdict">{E(d["verdict_line"])}</p>\n')
    w('<div class="pair">\n')
    w(f'<figure><figcaption>mermaid.js 12.2.0</figcaption><div class="canvas"><img src="img/{n}.mermaid.webp" width="{mw}" height="{mh}" alt="{n} dirender mermaid" loading="lazy"></div></figure>\n')
    w(f'<figure><figcaption>flowmaid 0.40.2</figcaption><div class="canvas"><img src="img/{n}.fm0402.webp" width="{nw}" height="{nh}" alt="{n} dirender flowmaid 0.40.2" loading="lazy"></div></figure>\n')
    w('</div>\n<div class="notes"><div><h4>Isu 30 Okt, sekarang</h4><ul class="olist">')
    for o in d['old_issues']:
        w(f'<li>{chip(o["status"])}<span><span class="oi">{E(o["issue"])}</span>{(" <span class=onote>" + E(o["note"]) + "</span>") if o.get("note") else ""}</span></li>')
    w('</ul></div><div>')
    if d['remaining']:
        w('<h4>Masih terlihat di 0.40.2</h4><ul>' + ''.join(f'<li>{E(x)}</li>' for x in d['remaining']) + '</ul>')
    w(f'<h4>Flowmaid lebih baik</h4><p>{E(d["better"])}</p></div></div>\n')
    w(f'<details class="was"><summary>Lihat gambar 0.32 (30 Okt) · penilaian waktu itu: {E(c["verdict032"])}</summary><div class="canvas"><img src="img/{n}.flowmaid.webp" width="{ow}" height="{oh}" alt="{n} dirender flowmaid 0.32" loading="lazy"></div></details>\n')
    w('</article>\n')
w('  </div>\n</section>\n')

w(open(os.path.join(F, 'tools/tail.html')).read())
os.makedirs(os.path.join(F, 'page'), exist_ok=True)
open(os.path.join(F, 'page/index.html'), 'w').write(''.join(out))
print('ok', len(''.join(out)), vc, oi, 'reasons beres', rb, 'bugs beres', bb)
