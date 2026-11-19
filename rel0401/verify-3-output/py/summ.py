# summ.py results.tsv... : counts by output class and flags
import collections,sys
st=collections.Counter(); flags=collections.Counter(); ex={}
newerr=[]; tryb=[]; files=set(); kinds=collections.Counter()
for path in sys.argv[1:]:
  for l in open(path):
    r=l.rstrip('\n').split('\t')
    if len(r)<3: continue
    f,k,s=r[0],r[1],r[2]; info=r[3] if len(r)>3 else ''
    files.add(f)
    if k=='time': continue
    if k=='try':
        if s and not s.endswith('OK'): tryb.append((f,s[:300]))
        st['try:'+('none' if not s else 'OK' if s.endswith('OK') else 'BAD')]+=1
        continue
    kk = k if k in ('render','scene') else 'routed'
    head=s.split(' ')[0]
    if head=='DIFF': head='DIFF:'+s.split(' ')[1]
    st[(kk,head)]+=1
    if s.startswith('NEWERR'): newerr.append((f,s))
    if s.startswith(('NOWOK','ERRDIFF')) or 'PANIC' in s or head.startswith('DIFF:vb00') : ex.setdefault(s[:60],(f,k,s[:300]))
    for t in info.split(';'):
        if t and t[0].isupper():
            key=t.split(' ')[0].split('=')[0]; flags[key]+=1; ex.setdefault(key,(f,k,info[:300]))
    if kk=='routed':
        vb0 = ';vb=0,0' in info or info.startswith('vb=0,0')
        kinds['routed vb00' if (';vb=0,0' in info or 'vb=0,0;' in info) else 'routed vb!=0']+=1
print('files',len(files))
for k,v in sorted(st.items(),key=str): print(' ',k,v)
print('flags',dict(flags))
print('kinds',dict(kinds))
print('newerr',len(newerr))
for k,v in ex.items(): print('EX',k,v)
for t in tryb[:20]: print('TRYBAD',t)
open('/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/verify-3-output/newerr.tsv','w').write(''.join(f'{a}\t{b}\n' for a,b in newerr))
