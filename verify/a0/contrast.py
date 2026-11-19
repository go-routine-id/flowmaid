def lum(c):
    c=c.lstrip('#')
    if len(c)==3: c=''.join(x*2 for x in c)
    r,g,b=[int(c[i:i+2],16)/255 for i in (0,2,4)]
    f=lambda v: v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4
    return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b)
def cr(a,b):
    la,lb=lum(a),lum(b); return (max(la,lb)+0.05)/(min(la,lb)+0.05)
ACC=["#5b6dc0","#33a35c","#d99114","#c2588c","#199a8e","#8a5cd6","#d96459","#4d8fbf"]
INK=["#5b6dc0","#2a864c","#a16b0f","#b85485","#15847a","#8a5cd6","#be574e","#437ca5"]
DEEP=["#3b477d","#216a3c","#8d5e0d","#7e395b","#10645c","#5a3c8b","#8d413a","#325d7c"]
DARK="#0a0e1a"
for a,i,d in zip(ACC,INK,DEEP):
    w=cr(a,"#ffffff"); dk=cr(a,DARK); t1=cr(a,"#111827")
    print(f"{a} white {w:.2f} dark {dk:.2f} #111827 {t1:.2f}  ink {i} on white {cr(i,'#ffffff'):.2f}  deep {d} white {cr(d,'#ffffff'):.2f}")
# is ink minimal darkening? check one step lighter (scale +1%)
def scale(c,k):
    c=c.lstrip('#'); return '#'+''.join(f"{min(255,round(int(c[i:i+2],16)*k)):02x}" for i in (0,2,4))
print("note text", cr("#232840","#fff8c5"), "note border", cr("#9c8a2c","#fff8c5"))
