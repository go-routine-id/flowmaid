import colorsys, math
A=["#5b6dc0","#33a35c","#d99114","#c2588c","#199a8e","#8a5cd6","#d96459","#4d8fbf"]
N=["indigo","green","amber","rose","teal","violet","coral","steel"]
def rgb(h): h=h.lstrip('#'); return tuple(int(h[i:i+2],16) for i in (0,2,4))
def lin(c):
    c/=255
    return c/12.92 if c<=0.04045 else ((c+0.055)/1.055)**2.4
def L(h):
    r,g,b=map(lin,rgb(h)); return 0.2126*r+0.7152*g+0.0722*b
def cr(a,b):
    la,lb=L(a),L(b); hi,lo=max(la,lb),min(la,lb); return (hi+0.05)/(lo+0.05)
for ink in ["#111827","#0f172a","#0b1120","#0a0f1c","#101522","#0d1117","#000000","#1a1a2e","#121826"]:
    print(ink, round(L(ink),5), [round(cr(a,ink),2) for a in A])
print("white",[round(cr(a,"#ffffff"),2) for a in A])
for a,n in zip(A,N): print(n,a,round(L(a),4))
print()
for ink in ["#0b1020","#0a0f1e","#0b0f19","#0a0e1a","#090d1a","#0c1222","#0a1020","#080c18"]:
    print(ink, round(L(ink),5), min(cr(a,ink) for a in A if cr(a,"#ffffff")<4.5), round(cr(ink,"#232840"),2))
