from design import *
fills=['#cdffb2','#bdf3a0','#aee78f','#9fdb7e']
strokes=[]; parent='#ffffff'
for f in fills:
    strokes.append(darken_to('#6eaa49',['#ffffff',parent,f])); parent=f
check_clusters('forest2','#ffffff',fills,strokes,'#000000')
for f in fills: print(f, 'vs node', round(cr(f,'#cde498'),3))
