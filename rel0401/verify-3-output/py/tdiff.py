# tdiff.py dev.svg head.svg : list numbers that are not translated by the head viewBox origin
import re,sys
NUM=re.compile(r'(?<![A-Za-z_#])-?\d+(?:\.\d+)?')
def vb(s):
    m=re.search(r' viewBox="([^"]+)"',s); return [float(x) for x in m.group(1).split()]
d=open(sys.argv[1]).read(); h=open(sys.argv[2]).read()
ox,oy=vb(h)[:2]
dl=d.split('\n'); hl=h.split('\n')
print('lines',len(dl),len(hl),'origin',ox,oy)
n=0
for i,(a,b) in enumerate(zip(dl,hl)):
    if a==b or a.startswith('<svg'): continue
    na=[float(x) for x in NUM.findall(a)]; nb=[float(x) for x in NUM.findall(b)]
    ta=NUM.sub('#',a); tb=NUM.sub('#',b)
    if b.startswith('<rect x=') and a.startswith('<rect width='): continue
    if len(na)!=len(nb) or ta!=tb:
        print('STRUCT line',i); print(' D',a[:400]); print(' H',b[:400]); n+=1; continue
    bad=[(j,u,v) for j,(u,v) in enumerate(zip(na,nb)) if abs(v-u)>1e-9 and abs(v-u-ox)>0.151 and abs(v-u-oy)>0.151]
    if bad:
        n+=1
        print('line',i,'bad',bad[:8])
        # show context around first bad token
        print(' D',a[:300]); print(' H',b[:300])
    if n>6: break
