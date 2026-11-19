import sys,re
def secs(p):
    out={}; cur='scene'; buf=[]
    for l in open(p):
        if l.startswith('--'):
            out[cur]=''.join(buf); cur=l.strip(); buf=[]
        else: buf.append(l)
    out[cur]=''.join(buf); return out
a=secs(sys.argv[1]); b=secs(sys.argv[2])
print(sys.argv[1].split('/')[-1], [k for k in a if a.get(k)!=b.get(k)])
