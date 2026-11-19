import re, sys, os, glob, csv
from collections import Counter
V=sys.argv[1]; R=sys.argv[2]
rel=re.compile(r'^\s*([\w.`~<>-]*?[\w`>])\s*(?:"[^"]*")?\s*(<\|--|--\|>|\*--|--\*|o--|--o|<-->|-->|<--|--|\.\.>|<\.\.|\.\.\|>|<\|\.\.|\*\.\.|\.\.\*|o\.\.|\.\.o|\.\.)\s*(?:"[^"]*")?\s*([\w`][\w.`~<>]*)')
def ml(p):
    c=Counter()
    for ln in open(p,encoding='utf-8',errors='replace').read().splitlines():
        m=rel.match(ln)
        if m and m.group(1)==m.group(3): c[m.group(1)]+=1
    return max(c.values()) if c else 0
for s in ['cfz2','cfz','lp']:
    d=Counter(ml(p) for p in glob.glob(f'{V}/r2/{s}/*.mmd'))
    print(s, sorted(d.items()))
# pooled over the three generated sets
tp=tn=tf=0
for s in ['cfz2','cfz','lp']:
    for b,acc in (('hp','p'),('hn','n')):
        for r in csv.DictReader(open(f'{R}/fl-{s}-{b}.tsv'),delimiter='\t'):
            if b=='hp': tp+=int(r['flipframes']); tf+=int(r['frames'])
            else: tn+=int(r['flipframes'])
print('pooled generated 2,132: frames',tf,'%.2f%% -> %.2f%%'%(100*tp/tf,100*tn/tf))
