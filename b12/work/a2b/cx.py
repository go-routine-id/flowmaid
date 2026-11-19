import csv,sys
names=['infra-overview','infra-overview-v2','perm-a-edges-reversed','perm-b-subgraphs-reversed','perm-c-actors-reversed','perm-d-edges-rotated','perm-e-one-more-inner-edge','blocks-overview','blocks-overview-bt','blocks-overview-lr','blocks-overview-rl','dense-td','dense-lr','bench-75']
print('must            ', '20 17 20 23 20 20 22 7 7 7 7 74 51 325')
for lab in sys.argv[1:]:
    rows={r['name']:r for r in csv.DictReader(open(f'/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/local/out/{lab}/metrics.tsv'),delimiter='\t')}
    print(f"{lab:16s}", ' '.join(rows[n]['crossings'] for n in names), '| W', ' '.join(rows[n]['W'] for n in names[:8]))
