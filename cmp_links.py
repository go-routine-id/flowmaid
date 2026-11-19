import sys
def load(f):
    d={}
    for l in open(f):
        if l.startswith('LINK '):
            _,n,e,k,dev=l.split()
            d[(n,e)]=(k,float(dev))
    return d
b=load(sys.argv[1])
for f in sys.argv[2:]:
    h=load(f)
    lost=sum(1 for k,v in b.items() if v[0]=='EE' and h.get(k,('',0))[0]=='PP')
    lost_list=[k for k,v in b.items() if v[0]=='EE' and h.get(k,('',0))[0]=='PP']
    mism=sum(1 for v in h.values() if v[0] in('EP','PE'))
    big=sum(1 for v in h.values() if v[1]>20)
    ee=sum(1 for v in h.values() if v[0]=='EE')
    agreed_now_far=sum(1 for k,v in b.items() if v[1]<1 and h.get(k,('',0))[1]>20)
    eediff=sum(1 for v in h.values() if v[0]=='EE' and v[1]>1)
    print(f, 'EE',ee,'lost',lost,'mismatch',mism,'>20px',big,'agreed->far',agreed_now_far,'EE-differing>1px',eediff)
    if len(sys.argv)==3: print(lost_list[:20])
