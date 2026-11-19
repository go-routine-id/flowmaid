# classify.py <diffsdir> : classify every dev/head non-translation pair
import re,sys,os,collections
NUM=re.compile(r'(?<![A-Za-z_#])-?\d+(?:\.\d+)?')
def vb(s):
    m=re.search(r' viewBox="([^"]+)"',s); return [float(x) for x in m.group(1).split()]
D=sys.argv[1]
cls=collections.Counter(); ex={}
for f in sorted(os.listdir(D)):
    if not f.endswith('.dev.svg'): continue
    d=open(os.path.join(D,f)).read(); h=open(os.path.join(D,f.replace('.dev.','.head.'))).read()
    ox,oy=vb(h)[:2]
    dl=d.split('\n'); hl=h.split('\n')
    kinds=set()
    if len(dl)!=len(hl): kinds.add('linecount')
    for a,b in zip(dl,hl):
        if a==b or a.startswith('<svg'): continue
        if b.startswith('<rect x=') and a.startswith('<rect width='): continue
        na=[float(x) for x in NUM.findall(a)]; nb=[float(x) for x in NUM.findall(b)]
        if len(na)!=len(nb) or NUM.sub('#',a)!=NUM.sub('#',b):
            tag=a.split(' ')[0]
            kinds.add('struct:'+tag+(':'+a.split('font-size="')[1][:2] if 'font-size' in a else '')); continue
        # find attr name of each number
        for m1,m2 in zip(NUM.finditer(a),NUM.finditer(b)):
            u=float(m1.group()); v=float(m2.group())
            if abs(v-u)<1e-9 or abs(v-u-ox)<=0.151 or abs(v-u-oy)<=0.151: continue
            pre=a[:m1.start()]
            am=re.findall(r'([a-zA-Z-]+)="[^"]*$',pre)
            att=am[-1] if am else '?'
            tag=re.findall(r'<([a-z]+)[^<]*$',pre)
            tg=tag[-1] if tag else '?'
            if abs(v-u)<=0.11: kinds.add(f'tiny:{tg}.{att}')
            elif min(abs(v-u-ox),abs(v-u-oy))<=0.25: kinds.add(f'near-trans:{tg}.{att}')
            else:
                kinds.add(f'moved:{tg}.{att}'+(':card' if 'font-size="11"' in a else ''))
    k=' '.join(sorted(kinds)); cls[k]+=1; ex.setdefault(k,f)
for k,v in cls.most_common(): print(v,k,'eg',ex[k])
