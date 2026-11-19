# Check ER SVGs: label rects vs foreign relationship lines, glyphs, tables, canvas.
import re, sys
def nums(s): return [float(x) for x in re.findall(r'-?\d+(?:\.\d+)?', s)]
def attr(l, a):
    m = re.search(r'\s%s="([^"]*)"' % a, l); return m.group(1) if m else None
def sample(d):
    toks = re.findall(r'[MLCQZ]|-?\d+(?:\.\d+)?', d)
    pts=[]; i=0; cur=None; cmd=None
    while i < len(toks):
        t=toks[i]
        if t in 'MLCQZ': cmd=t; i+=1; continue
        if cmd=='M': cur=(float(toks[i]),float(toks[i+1])); pts.append(cur); i+=2; cmd='L'
        elif cmd=='L':
            p=(float(toks[i]),float(toks[i+1])); i+=2
            for k in range(1,11): pts.append((cur[0]+(p[0]-cur[0])*k/10, cur[1]+(p[1]-cur[1])*k/10))
            cur=p
        elif cmd=='C':
            a=(float(toks[i]),float(toks[i+1])); b=(float(toks[i+2]),float(toks[i+3])); p=(float(toks[i+4]),float(toks[i+5])); i+=6
            for k in range(1,25):
                t_=k/24; u=1-t_
                pts.append((u**3*cur[0]+3*u*u*t_*a[0]+3*u*t_*t_*b[0]+t_**3*p[0], u**3*cur[1]+3*u*u*t_*a[1]+3*u*t_*t_*b[1]+t_**3*p[1]))
            cur=p
        elif cmd=='Q':
            a=(float(toks[i]),float(toks[i+1])); p=(float(toks[i+2]),float(toks[i+3])); i+=4
            for k in range(1,13):
                t_=k/12; u=1-t_
                pts.append((u*u*cur[0]+2*u*t_*a[0]+t_*t_*p[0], u*u*cur[1]+2*u*t_*a[1]+t_*t_*p[1]))
            cur=p
        else: i+=1
    return pts
def inside(p, r, pad=0): return r[0]-pad < p[0] < r[2]+pad and r[1]-pad < p[1] < r[3]+pad
for f in sys.argv[1:]:
    L = open(f).read().splitlines()
    W = float(attr(L[0],'width')); H=float(attr(L[0],'height'))
    rels=[]; labels=[]; tables=[]; glyph_pts=[]; circles=[]
    seen_table=False
    for l in L:
        if l.startswith('<path') and 'stroke-width="1.7"' in l and 'fill="none"' in l:
            d=attr(l,'d'); n=nums(d)
            if not seen_table and not (len(n)==4 and ' L ' in d): rels.append(sample(d))
            elif len(n)==4: glyph_pts.append(sample(d))
        elif l.startswith('<circle'):
            circles.append((float(attr(l,'cx')),float(attr(l,'cy')),float(attr(l,'r'))))
        elif l.startswith('<rect') and 'height="20"' in l and 'stroke="#d5d9ec"' in l:
            x,y,w,h=[float(attr(l,k)) for k in ('x','y','width','height')]; labels.append((x,y,x+w,y+h))
        elif l.startswith("<rect") and ("fill=\"none\" stroke=\"#" in l or ("fill=\"#ffffff\" stroke=\"#" in l and "stroke-width=\"1.6\"" in l)):
            seen_table=True
            x,y,w,h=[float(attr(l,k)) for k in ('x','y','width','height')]; tables.append((x,y,x+w,y+h))
        elif l.startswith('<rect') and 'fill="#ffffff"/>' in l and 'rx="4"' in l:
            seen_table=True
    issues=[]
    for li,r in enumerate(labels):
        hit=[j for j,ps in enumerate(rels) if any(inside(p,r) for p in ps)]
        if len(hit)>=2: issues.append(f"label{li} {r} crossed by {len(hit)} lines {hit}")
        for t in tables:
            if r[0]<t[2] and t[0]<r[2] and r[1]<t[3] and t[1]<r[3]: issues.append(f"label{li} overlaps table {t}")
        for g in glyph_pts:
            if any(inside(p,r) for p in g): issues.append(f"label{li} covers glyph stroke"); break
        for c in circles:
            dx=max(r[0]-c[0],0,c[0]-r[2]); dy=max(r[1]-c[1],0,c[1]-r[3])
            if (dx*dx+dy*dy)**.5 < c[2]: issues.append(f"label{li} covers circle {c}")
        if r[0]<0 or r[1]<0 or r[2]>W or r[3]>H: issues.append(f"label{li} {r} outside canvas {W}x{H}")
        for lj,o in enumerate(labels):
            if lj>li and r[0]<o[2] and o[0]<r[2] and r[1]<o[3] and o[1]<r[3]: issues.append(f"label{li} overlaps label{lj}")
    # lines through tables (excluding ends near border)
    for j,ps in enumerate(rels):
        for t in tables:
            n=sum(1 for p in ps if inside(p,t,-3))
            if n>0: issues.append(f"line{j} passes through table {t} ({n} pts)")
    allp=[p for ps in rels for p in ps]+[p for g in glyph_pts for p in g]
    for p in allp:
        if p[0]<0 or p[1]<0 or p[0]>W or p[1]>H: issues.append(f"stroke point {p} outside canvas"); break
    print(f"{f.split('/')[-1]}: {len(rels)} rels {len(labels)} labels {len(tables)} tables -> {len(issues)} issues")
    for i in issues: print("   ", i)
