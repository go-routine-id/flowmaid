import re,math,sys
sys.path.insert(0,'.')
from geo import sample
s=open(sys.argv[1]).read()
s2=re.sub(r'<defs>.*?</defs>','',s,flags=re.S)
cnt={'ortho':0,'single':0,'spline':0}; ex={'spline':[]}
for l in s2.split('\n'):
    if '<path' in l and 'marker-end' in l:
        t=re.search(r'<title>(.*?)</title>',l).group(1); d=re.search(r' d="([^"]*)"',l).group(1)
        if d.count('C')==1 and 'L' not in d: cnt['single']+=1; continue
        P=sample(d,1); al=0; tot=0
        for k in range(len(P)-1):
            dx=P[k+1][0]-P[k][0]; dy=P[k+1][1]-P[k][1]; L=math.hypot(dx,dy); tot+=L
            if L>0 and (abs(dx)/L>0.999 or abs(dy)/L>0.999): al+=L
        if al/tot>0.85: cnt['ortho']+=1
        else: cnt['spline']+=1; ex['spline'].append((t,round(al/tot,2)))
print(cnt); print(ex)
