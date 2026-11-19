A=["#5b6dc0","#33a35c","#d99114","#c2588c","#199a8e","#8a5cd6","#d96459","#4d8fbf","#3b477d","#216a3c","#8d5e0d","#7e395b","#10645c","#5a3c8b","#8d413a","#325d7c"]
N=["indigo","green","amber","rose","teal","violet","coral","steel","dIndigo","dGreen","dAmber","dRose","dTeal","dViolet","dCoral","dSteel"]
def lab(c):
    r,g,b=[int(c[i:i+2],16)/255 for i in (1,3,5)]
    f=lambda v: v/12.92 if v<=0.04045 else ((v+0.055)/1.055)**2.4
    r,g,b=f(r),f(g),f(b)
    x=(0.4124*r+0.3576*g+0.1805*b)/0.95047; y=0.2126*r+0.7152*g+0.0722*b; z=(0.0193*r+0.1192*g+0.9505*b)/1.08883
    ff=lambda t: t**(1/3) if t>216/24389 else (24389/27*t+16)/116
    fx,fy,fz=ff(x),ff(y),ff(z)
    return (116*fy-16,500*(fx-fy),200*(fy-fz))
L=[lab(c) for c in A]
pairs=[]
for i in range(16):
  for j in range(i+1,16):
    d=sum((L[i][k]-L[j][k])**2 for k in range(3))**.5
    pairs.append((d,N[i],N[j]))
pairs.sort()
for p in pairs[:30]: print("%.1f %s %s"%p)
