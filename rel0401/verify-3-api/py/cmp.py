# compare base/head TSV; write diffs; print stats as one JSON line. usage: cmp.py base.tsv head.tsv diffs.tsv
import sys, json, collections
b=[l.rstrip('\n').split('\t') for l in open(sys.argv[1]) if l.strip()]
h=[l.rstrip('\n').split('\t') for l in open(sys.argv[2]) if l.strip()]
st=collections.Counter(); diffs=[]
if len(b)!=len(h): st['LEN MISMATCH']+=1
for x,y in zip(b,h):
    if x[:2]!=y[:2]: st['ROW MISMATCH']+=1; continue
    if x[1]=='PANIC' or y[1]=='PANIC': st['panic '+('b' if x[1]=='PANIC' else '')+('h' if y[1]=='PANIC' else '')]+=1; continue
    if x[2]=='ERR' or y[2]=='ERR':
        st['err same' if x==y else 'err DIFF']+=1
        if x!=y: diffs.append(['ERR']+x+y[2:])
        continue
    cat='static' if x[1] in ('render','scene') else 'routed'
    same=x[2]==y[2]; hz=y[3].startswith('0 0 ')
    st[f"{cat} {'same' if same else 'diff'} {'vb00' if hz else 'vbX'}"]+=1
    if not same:
        diffs.append([cat]+x+y[2:])
        if cat=='routed' and not hz:
            v=list(map(float,y[3].split())); bv=list(map(float,x[3].split()))
            if not (v[0]<0 or v[1]<0): st['routed diff vbX NONNEG']+=1
            if v[2:]!=bv[2:]: st['routed diff size changed']+=1
open(sys.argv[3],'w').write(''.join('\t'.join(d)+'\n' for d in diffs))
print(json.dumps(st))
