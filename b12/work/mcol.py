import csv,sys
# usage: mcol.py names(comma) cols(comma) dirs...
names=sys.argv[1].split(','); cols=sys.argv[2].split(',')
B='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/local/out/'
for n in names:
    print('==',n)
    for d in sys.argv[3:]:
        p=d if '/' in d else B+d+'/metrics.tsv'
        rows={r['name']:r for r in csv.DictReader(open(p),delimiter='\t')}
        r=rows.get(n)
        if not r: print(f"  {d:12s} -"); continue
        print(f"  {d[-14:]:14s}", ' '.join(f"{c}={r.get(c,'?')}" for c in cols))
