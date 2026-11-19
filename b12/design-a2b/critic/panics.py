import sys,re
# print the panic message lines of every failed test from a cargo test log, normalised
def panics(p):
    out={}
    lines=open(p,errors='replace').read().split('\n')
    cur=None;buf=[]
    for l in lines:
        m=re.match(r"thread '([^']+)' \(\d+\) panicked at",l)
        if m:
            cur=m.group(1);buf=[];continue
        if cur is not None:
            if l.startswith('stack backtrace') or l.startswith('note:') or l.startswith('----') or re.match(r'\s+\d+: ',l):
                out[cur]=buf;cur=None;continue
            buf.append(l)
    return out
a=panics(sys.argv[1]);b=panics(sys.argv[2])
for t in sys.argv[3:]:
    A=set(a.get(t,[]));B=set(b.get(t,[]))
    print('== %s: only-base %d only-variant %d'%(t,len(A-B),len(B-A)))
    for l in sorted(A-B): print('  - '+l[:230])
    for l in sorted(B-A): print('  + '+l[:230])
