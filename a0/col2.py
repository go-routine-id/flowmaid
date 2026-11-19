exec(open('col.py').read().split('for ink in')[0])
def hexs(t): return "#%02x%02x%02x"%tuple(max(0,min(255,round(v))) for v in t)
def scale(h,k): return hexs(tuple(c*k for c in rgb(h)))
def lab(h):
    r,g,b=map(lin,rgb(h))
    X=(0.4124*r+0.3576*g+0.1805*b)/0.95047; Y=(0.2126*r+0.7152*g+0.0722*b); Z=(0.0193*r+0.1192*g+0.9505*b)/1.08883
    f=lambda t: t**(1/3) if t>216/24389 else (24389/27*t+16)/116
    fx,fy,fz=f(X),f(Y),f(Z)
    return (116*fy-16, 500*(fx-fy), 200*(fy-fz))
def de(a,b):
    return math.dist(lab(a),lab(b))
import itertools
print("base min dE", min((de(a,b),a,b) for a,b in itertools.combinations(A,2)))
# inks: darken until white >= 4.5
inks=[]
for a in A:
    k=1.0
    while cr(scale(a,k),"#ffffff")<4.5: k-=0.005
    inks.append(scale(a,k)); 
print("inks",inks,[round(cr(i,'#ffffff'),2) for i in inks])
for k in [0.62,0.65,0.68,0.7,0.72]:
    deep=[scale(a,k) for a in A]
    allc=A+deep
    m=min((de(a,b),a,b) for a,b in itertools.combinations(allc,2))
    print(k,deep,"min dE",m, "white",[round(cr(d,'#ffffff'),2) for d in deep])
