import sys,re,glob,csv
# usage: hopcount.py <svg dir> <metrics.tsv>: flowmaid hops drawn (pairs of arcs in edge paths) vs bridged by blockmetrics (hops)
d=sys.argv[1]
rows={r[0]:r for r in csv.reader(open(sys.argv[2]),delimiter='\t')}
h=rows['name']; hi=h.index('hops')
tot_a=0; tot_h=0; files=[]
for f in sorted(glob.glob(d+'/*.svg')):
    s=open(f).read(); n=f.split('/')[-1][:-4]
    paths=re.findall(r'<path d="([^"]*)"[^>]*><title>',s)
    a=sum(len(re.findall(r'A ',p)) for p in paths)
    hops=int(rows[n][hi]) if n in rows else 0
    tot_a+=a; tot_h+=hops
    if a//2!=hops: files.append((n,a//2,hops))
print(d,'drawn hops (arcs/2) =',tot_a//2,'bridged =',tot_h,'files differing',len(files), files[:14])
