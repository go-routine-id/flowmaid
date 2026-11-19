from cr import *
def check_clusters(name, bg, fills, strokes, title):
    parent=bg
    for d,(f,s) in enumerate(zip(fills,strokes)):
        r=[round(cr(s,x),2) for x in (bg,parent,f)]
        print(name,d,f,s,'border',r,'step',round(cr(f,parent),3),'title',round(cr(title,f),2))
        parent=f
def darken_to(c, others, target=3.0):
    k=1.0
    while k>0:
        x=scale(c,k)
        if all(cr(x,o)>=target for o in others): return x
        k-=0.01
def lighten_to(c, others, target=3.0):
    t=0.0
    while t<=1:
        x=mixw(c,t)
        if all(cr(x,o)>=target for o in others): return x
        t+=0.01
# Mermaid default clusters
fills=['#ffffde','#f9f9cb','#f3f3b8','#ededa5']
strokes=[]; parent='#ffffff'
for f in fills:
    strokes.append(darken_to('#aaaa33',['#ffffff',parent,f])); parent=f
check_clusters('mermaid','#ffffff',fills,strokes,'#333333')
print('note', darken_to('#aaaa33',['#fff5ad','#ffffff']))
# neutral
fills=['#f7f7f7','#eeeeee','#e5e5e5','#dcdcdc']
strokes=[]; parent='#ffffff'
for f in fills:
    strokes.append(darken_to('#8a8a8a',['#ffffff',parent,f])); parent=f
check_clusters('neutral','#ffffff',fills,strokes,'#333333')
# forest
fills=['#e6ffd9','#cdffb2','#bdf2a0','#addf8f']
fills=['#ebffe0','#dcf9cb','#cdf2b6','#bfeba2']
strokes=[]; parent='#ffffff'
for f in fills:
    strokes.append(darken_to('#6eaa49',['#ffffff',parent,f])); parent=f
check_clusters('forest','#ffffff',fills,strokes,'#000000')
print('forest note', darken_to('#6eaa49',['#fff5ad','#ffffff']))
# dark
bg='#333333'
fills=['#3d3f3f','#474949','#515353','#5b5d5d']
strokes=[]; parent=bg
for f in fills:
    strokes.append(lighten_to('#8a8a8a',[bg,parent,f])); parent=f
check_clusters('dark',bg,fills,strokes,'#f0f0f0')
for a,b in [('#e6e6e6','#333333'),('#e6e6e6','#1f2020'),('#b8b8b8','#1f2020'),('#b8b8b8','#333333'),('#d3d3d3','#333333'),('#8f8f8f','#333333'),('#cccccc','#1f2020'),('#f4f4f4','#333333'),('#888888','#ffffff'),('#666666','#ffffff'),('#6b6b6b','#333333')]:
    print(a,b,round(cr(a,b),2))
print('dark note border', lighten_to('#9c8a2c',['#4d4a2e','#333333']), cr('#f2f2f2','#4d4a2e'))
