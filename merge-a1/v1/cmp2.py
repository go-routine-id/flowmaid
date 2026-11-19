import sys, collections
V='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1'
def load(ps):
    d={}
    for p in ps:
        try: ls=open(V+'/'+p).read().split('\n')
        except FileNotFoundError: print('missing file',p); continue
        if ls and ls[-1]!='': ls=ls[:-1]   # drop a partial last line
        for l in ls:
            if not l: continue
            k,_,v=l.partition('\t')
            d.setdefault(k,v)
    return d
mode=sys.argv[1]
if mode=='def':
    A=load(['fullA-dev.00.tsv','full-dev.01.tsv','restS-dev.0.tsv','restS-dev.1.tsv','restBa-dev.tsv','restBb-dev.tsv'])
    B=load(['fullA-head.00.tsv','full-head.01.tsv','restS-head.0.tsv','restS-head.1.tsv','restBa-head.tsv','restBb-head.tsv'])
else:
    A=load(['on-a1.0.tsv','on1a-a1.tsv','on1b-a1.00.tsv','on1b-a1.01.tsv','onB-a1.tsv'])
    B=load(['on-head.0.tsv','on1a-head.tsv','on1b-head.00.tsv','on1b-head.01.tsv','onB-head.tsv'])
inputs=[l for l in open(V+'/inputs.txt').read().split('\n') if l]
both=[k for k in inputs if k in A and k in B]
diff=[k for k in both if A[k]!=B[k]]
geo=sum(1 for k in both if 's.svg=' in B[k]); full=sum(1 for k in both if 'p2r.svg=' in B[k]); mid=sum(1 for k in both if 'p0r=' in B[k]); basic=sum(1 for k in both if B[k].count('=')==1)
err=sum(1 for k in both if 'ERR:' in B[k]); panic=sum(1 for k in both if 'PANIC' in B[k])
print(f'{mode}: inputs {len(inputs)} compared {len(both)} differ {len(diff)} | flowchart/state with geometry {geo} (full {full}, mid {mid}) svg-only {basic} parse-errors {err} panics {panic}')
if mode=='on':
    dev=load(['fullA-dev.00.tsv','full-dev.01.tsv','restS-dev.0.tsv','restS-dev.1.tsv','restBa-dev.tsv','restBb-dev.tsv'])
    kinds=collections.Counter()
    notdev=[]
    for k in diff:
        first=open(k).read().lstrip().split('\n')[0].split()
        kinds[first[0] if first else '?']+=1
        if dev.get(k)!=B[k]: notdev.append(k)
    print(' diff kinds',dict(kinds),'; block-mode head rows equal to dev default:',len(diff)-len(notdev),'of',len(diff))
    for k in notdev[:10]: print('  NOT-DEV',k.split('scratchpad/')[-1])
    sub=sum(1 for k in both if 's.svg=' in B[k] and B[k]!=dev.get(k))
    print(' flowchart/state inputs whose block-mode drawing differs from the default drawing (block mode active):',sub)
else:
    for k in diff[:10]: print('DIFF',k)
