exec(open('col2.py').read().split('import itertools')[0])
import itertools
deep=[scale(a,0.65) for a in A]
print(deep)
for fill,stroke in [("#fff6c7","#a8902a"),("#fff8c5","#9c8a2c"),("#fef7d6","#a38b2b"),("#fff5ad","#aaaa33"),("#fdf6cf","#9a8526"),("#fff7cc","#998524")]:
    print(fill,stroke,"stroke/fill",round(cr(stroke,fill),2),"stroke/white",round(cr(stroke,'#ffffff'),2),"text",round(cr('#232840',fill),2),
      "min dE stroke vs accents",round(min(de(stroke,a) for a in A+deep),1),"fill vs amber tint", round(de(fill,'#fcf2da'),1))
