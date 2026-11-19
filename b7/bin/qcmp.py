import sys
cols = sys.argv[3].split(',') if len(sys.argv) > 3 else ['defects','border_len','overlap_len','tight_len','hug_len','band_pairs','band_len','min_edge_gap','lbl_edge','idiom_scubic','end_kink','x_drawn','edge_frame_pairs','height','width']
def load(p):
    rows = [r for r in (l.rstrip("\n").split("\t") for l in open(p) if l.strip()) if len(r) > 10]
    hdr = rows[0]
    out = {}
    for r in rows[1:]:
        out[(r[0], r[1])] = dict(zip(hdr, r))
    return out
a, b = load(sys.argv[1]), load(sys.argv[2])
print('file\tmode\t' + '\t'.join(cols))
for k in a:
    if k not in b: continue
    cells = []
    for c in cols:
        x, y = a[k].get(c, '?'), b[k].get(c, '?')
        try:
            fx, fy = float(x), float(y)
            cells.append(f'{fx:g}' if abs(fx - fy) < 1e-6 else f'{fx:g}->{fy:g}')
        except ValueError:
            cells.append('?')
    print('\t'.join(k) + '\t' + '\t'.join(cells))
