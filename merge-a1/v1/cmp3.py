import collections
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1'
exec(open(V+'/cmp2.py').read().split("mode=sys.argv[1]")[0].replace('import sys, collections',''))
A=load(['on-a1.0.tsv','on1a-a1.tsv','on1b-a1.00.tsv','on1b-a1.01.tsv','onB-a1.tsv'])
B=load(['on-head.0.tsv','on1a-head.tsv','on1b-head.00.tsv','on1b-head.01.tsv','onB-head.tsv'])
dev=load(['fullA-dev.00.tsv','full-dev.01.tsv','restS-dev.0.tsv','restS-dev.1.tsv','restBa-dev.tsv','restBb-dev.tsv'])
inputs=[l for l in open(V+'/inputs.txt').read().split('\n') if l]
fd=collections.Counter(); files=[]
for k in inputs:
    if k in A and k in B and A[k]!=B[k] and 's.svg=' in B[k]:
        fa=dict(x.split('=',1) for x in A[k].split('\t') if '=' in x); fb=dict(x.split('=',1) for x in B[k].split('\t') if '=' in x)
        d=tuple(sorted(x for x in fb if fa.get(x)!=fb.get(x)))
        fd[d]+=1; files.append(k)
        src=open(k).read()
for d,c in fd.most_common(): print(c,d)
open(V+'/onstate-diff.txt','w').write('\n'.join(files)+'\n')
bars=sum(1 for k in files if '<<fork>>' in open(k).read() or '<<join>>' in open(k).read())
dirn=sum(1 for k in files if 'direction' in open(k).read())
print('files',len(files),'with fork/join bars',bars,'with a direction',dirn)
import os
print(collections.Counter(os.path.dirname(k).split('scratchpad/')[-1] for k in files))
