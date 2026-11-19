import re,sys,math
s=open(sys.argv[1]).read()
s2=re.sub(r'<defs>.*?</defs>','',s,flags=re.S)
out=[]
for l in s2.split('\n'):
    if '<path' in l and 'marker-end' in l:
        t=re.search(r'<title>(.*?)</title>',l).group(1)
        d=re.search(r' d="([^"]*)"',l).group(1)
        nC=d.count('C'); nL=d.count('L')
        out.append((t,nC,nL,d))
for t,nC,nL,d in out:
    if nC==1 and nL==0: print('SINGLE',t,d)
print(len([o for o in out if o[1]==1 and o[2]==0]),'single-cubic of',len(out))
