import sys,csv
for f in sys.argv[1:]:
    rows=list(csv.DictReader(open(f),delimiter='\t'))
    for mode in ('static','routed'):
        tot={}
        for r in rows:
            if r['mode']!=mode: continue
            if r['file'] not in ('bench-75','dense-lr','dense-td','fanout-frames','back-edge','wide-labels'): continue
            for k in ('x_drawn','band_pairs','band_len','lat_wiggles','lat_backtrack'):
                if k in r:
                    tot[k]=tot.get(k,0)+float(r[k])
        print(f.split('/')[-1],mode,tot, [ (r['file'],r.get('x_drawn')) for r in rows if r['mode']==mode])
