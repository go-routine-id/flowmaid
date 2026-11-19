from design import *
bg='#1e1f22'
acc=["#5b6dc0","#33a35c","#d99114","#c2588c","#199a8e","#8a5cd6","#d96459","#4d8fbf"]
print('accents vs bg', [round(cr(a,bg),2) for a in acc])
fills=['#26282c','#2e3035','#36393e','#3e4147']
strokes=[]; parent=bg
for f in fills:
    strokes.append(lighten_to('#7a7d82',[bg,parent,f])); parent=f
check_clusters('dark',bg,fills,strokes,'#f0f0f0')
surf='#2b2d31'
for a,b in [('#e6e6e6',bg),('#e6e6e6',surf),('#b4b4b4',surf),('#b4b4b4',bg),('#d0d0d0',bg),('#d0d0d0',surf),('#8a8a8a',bg),('#c8c8c8',surf),('#c8c8c8',bg),('#f0f0f0',bg),('#5a5d63',surf),('#e6e6e6','#3e4147'), ('#f2f2f2','#3d3a26'),('#a69641','#3d3a26'),('#a69641',bg)]:
    print(a,b,round(cr(a,b),2))
for f in fills: print('node vs cluster', f, round(cr(surf,f),3), 'stroke', round(cr('#c8c8c8',f),2))
