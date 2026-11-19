import sys
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1'
def load(ps):
    d={}
    for p in ps:
        try: ls=open(p).read().split('\n')
        except FileNotFoundError: continue
        if ls and ls[-1]!='': ls=ls[:-1]
        for l in ls:
            if not l: continue
            k,_,v=l.partition('\t')
            if k in d and d[k]!=v: print('INCONSISTENT DUP',k)
            d.setdefault(k,v)
    return d
mode=sys.argv[1]
if mode=='def':
    A=load([V+'/fullA-dev.00.tsv',V+'/full-dev.01.tsv',V+'/restS-dev.0.tsv',V+'/restS-dev.1.tsv']); B=load([V+'/fullA-head.00.tsv',V+'/full-head.01.tsv',V+'/restS-head.0.tsv',V+'/restS-head.1.tsv'])
    Am=load([V+'/restB-dev.tsv']); Bm=load([V+'/restB-head.tsv'])
else:
    A=load([V+'/on-a1.0.tsv',V+'/on-a1.1.tsv']); B=load([V+'/on-head.0.tsv',V+'/on-head.1.tsv'])
    Am=load([V+'/onB-a1.tsv']); Bm=load([V+'/onB-head.tsv'])
inputs=[l for l in open(V+'/inputs.txt').read().split('\n') if l]
cov=0; diff=[]; geo=0; err=0; panic=0; missing=[]
for k in inputs:
    a=A.get(k) if k in A and k in B else (Am.get(k) if k in Am and k in Bm else None)
    b=B.get(k) if k in A and k in B else (Bm.get(k) if k in Am and k in Bm else None)
    if a is None: missing.append(k); continue
    cov+=1
    if a!=b: diff.append(k)
    if 's.svg=' in b: geo+=1
    if 'ERR:' in b: err+=1
    if 'PANIC' in b: panic+=1
print(f'inputs {len(inputs)} covered {cov} missing {len(missing)} differ {len(diff)} flow/state-geometry {geo} parse-errors {err} panics {panic}')
local=[k for k in inputs if '/b12/local/' in k]
print('local family covered',sum(1 for k in local if k not in missing),'of',len(local))
import collections
if mode!='def':
    kinds=collections.Counter()
    for k in diff:
        first=open(k).read().lstrip().split('\n')[0].split()
        kinds[first[0] if first else '?']+=1
    print('diff kinds',dict(kinds))
    dev=load([V+'/fullA-dev.00.tsv',V+'/full-dev.01.tsv',V+'/restS-dev.0.tsv',V+'/restS-dev.1.tsv'])
    devm=load([V+'/restB-dev.tsv'])
    eq=sum(1 for k in diff if (dev.get(k) or devm.get(k))==(B.get(k) or Bm.get(k)))
    print('block-mode diffs equal to dev default output:',eq,'of',len(diff))
    for k in diff:
        if (dev.get(k) or devm.get(k))!=(B.get(k) or Bm.get(k)): print('  NOT-DEV',k.split('scratchpad/')[-1])
else:
    for k in diff[:10]: print('DIFF',k)
