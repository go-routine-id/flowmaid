import math,re,csv
C='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/compare'
W='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/wt/rel39'
def tsv(p):
    r=list(csv.DictReader(open(p),delimiter='\t'))
    return {x['name']:x for x in r}
f=tsv(C+'/bench039/flowmaid.tsv'); m=tsv(C+'/bench039/mermaid.tsv')
f8=tsv(C+'/bench038/flowmaid.tsv'); m8=tsv(C+'/bench038/mermaid.tsv')
readme=open(W+'/README.md',encoding='utf-8').read()
rows=re.findall(r'^\| ([\w-]+) \| (\w+) \| ([\d.]+) \| ([\d.]+) \| (\d+)× \| ([\d.]+) \| ([\d.]+) \|$',readme,flags=re.M)
print(len(rows),'rows in README')
bad=0
for n,t,fm,mm,sp,fk,mk in rows:
    F=f[n];M=m[n]
    e_fm='%.2f'%float(F['median_ms']); e_mm='%.1f'%float(M['median_ms'])
    e_sp='%d'%round(float(M['median_ms'])/float(F['median_ms']))
    e_fk='%.1f'%(int(F['svg_bytes'])/1024); e_mk='%.1f'%(int(M['svg_bytes'])/1024)
    if (fm,mm,sp,fk,mk)!=(e_fm,e_mm,e_sp,e_fk,e_mk) or t!=F['type']:
        bad+=1;print('MISMATCH',n,(fm,mm,sp,fk,mk),(e_fm,e_mm,e_sp,e_fk,e_mk))
print('mismatches',bad, 'names equal', set(f)==set(r[0] for r in rows)==set(m))
def gm(f,m): 
    return math.exp(sum(math.log(float(m[n]['median_ms'])/float(f[n]['median_ms'])) for n in f)/len(f))
print('geomean 039',gm(f,m),'038',gm(f8,m8))
print('under 1ms',sum(float(f[n]['median_ms'])<1 for n in f))
rat=[int(m[n]['svg_bytes'])/int(f[n]['svg_bytes']) for n in f]
print('svg ratio min max',min(rat),max(rat))
import statistics
print('mermaid medians ratio geo', math.exp(sum(math.log(float(m[n]['median_ms'])/float(m8[n]['median_ms'])) for n in m)/len(m)), 'median', statistics.median(float(m[n]['median_ms'])/float(m8[n]['median_ms']) for n in m))
print('flowmaid medians ratio geo', math.exp(sum(math.log(float(f[n]['median_ms'])/float(f8[n]['median_ms'])) for n in f)/len(f)), 'median', statistics.median(float(f[n]['median_ms'])/float(f8[n]['median_ms']) for n in f))
for n in ['bench-75','dense-lr','dense-td']:
    print(n,f8[n]['svg_bytes'],f[n]['svg_bytes'],int(f[n]['svg_bytes'])/int(f8[n]['svg_bytes']))
