import csv,sys
# qcols.py file.tsv mode col1,col2,... [names...]: print the given harness columns
f,mode,cols=sys.argv[1],sys.argv[2],sys.argv[3].split(',')
names=sys.argv[4:]
rows=[r for r in csv.DictReader(open(f),delimiter='\t') if r['mode']==mode]
print('%-36s'%'file',' '.join('%9s'%c[:9] for c in cols))
for r in rows:
    if names and r['file'] not in names: continue
    if not names and all(r.get(c,'0') in ('0','0.0','0.00','NA') for c in cols): continue
    print('%-36s'%r['file'][:36],' '.join('%9s'%r.get(c,'?') for c in cols))
