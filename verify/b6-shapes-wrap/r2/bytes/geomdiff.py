import sys,re,subprocess
pre,now,listf=sys.argv[1:4]
num=re.compile(r'-?\d+\.?\d*')
def svg(b,f):
    return subprocess.run([b,f],capture_output=True,text=True).stdout
def nums(s):
    s=re.sub(r'flowmaid-[0-9a-f]+','H',s)
    return [float(x) for x in num.findall(s)], re.sub(r'-?\d+\.?\d*','#',s)
out=[]
for line in open(listf):
    cat,ty,f=line.rstrip('\n').split('\t')
    if cat!='geom': continue
    a=svg(pre,f); b=svg(now,f)
    na,sa=nums(a); nb,sb=nums(b)
    if sa!=sb or len(na)!=len(nb):
        out.append(('struct',ty,0.0,f))
    else:
        m=max((abs(x-y) for x,y in zip(na,nb)),default=0)
        out.append(('shift',ty,m,f))
import collections
c=collections.Counter((o[0],o[1]) for o in out); print(c)
sh=[o for o in out if o[0]=='shift']
for lim in [0.15,0.6,1.6,3,6,20,1e9]:
    print(lim, sum(1 for o in sh if o[2]<=lim))
with open(listf+'.geom','w') as fh:
    for o in sorted(out,key=lambda o:-o[2]): fh.write(f"{o[0]}\t{o[1]}\t{o[2]:.2f}\t{o[3]}\n")
