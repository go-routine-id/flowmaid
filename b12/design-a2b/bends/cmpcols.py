import sys,csv
# usage: cmpcols.py old.tsv new.tsv  -> every changed cell
def load(f):
    r=list(csv.reader(open(f),delimiter='\t')); h=r[0]; return h,{x[0]:dict(zip(h,x)) for x in r[1:]}
h,a=load(sys.argv[1]); _,b=load(sys.argv[2])
from collections import Counter
cnt=Counter(); rows=[]
for n in a:
    for c in h:
        if a[n][c]!=b[n][c]: cnt[c]+=1; rows.append((n,c,a[n][c],b[n][c]))
print('changed cells by column:',dict(cnt))
if len(sys.argv)>3:
    for r in rows:
        if r[1] in sys.argv[3].split(','): print('\t'.join(r))
