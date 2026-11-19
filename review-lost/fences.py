import re,sys
def f(p):
    s=open(p,encoding='utf-8').read()
    return re.findall(r'^```(\w*)\n(.*?)^```',s,flags=re.S|re.M)
o=f(sys.argv[1]); n=f(sys.argv[2])
oset={b for _,b in o}
print('old fences',len(o),'new fences',len(n))
for lang,b in n:
    print(lang or '(none)', 'IDENTICAL-TO-OLD' if b in oset else 'CHANGED/NEW', '|', b.splitlines()[0][:70])
