import re,collections
def load(p):
    d=collections.defaultdict(dict); pend=[]
    for line in open(p):
        if line.startswith('  '):
            m=re.match(r'  (S|R) X (\S+) x (\S+) : (\d+)(?: \[(\d+) (\d+)\])?',line)
            pend.append(m)
        elif not line.startswith('TOTAL'):
            f=line.split('\t')[0].split('/')[-1]
            for m in pend: d[(f,m.group(1))][(m.group(2),m.group(3))]=(int(m.group(4)),m.group(5),m.group(6))
            pend=[]
    return d
