import sys
sys.path.insert(0, '.')
import sim
ab = {'categories':'Ca','products':'Pr','order_items':'Oi','customers':'Cu','orders':'Or','payments':'Pa','shipments':'Sh'}
def ev(cols, rows, ox, oy, zoom_presses, folded):
    v = sim.View(); v.page_rows = rows - 9
    v.ox, v.oy = ox, oy
    for _ in range(zoom_presses): v.key('-')
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a); d, w = sim.defects(b, t, a)
    cl = [x for x in t if x['drawn'] != x['text']]
    full = [x['name'] for x in b if x['rect'] == x['full']]
    s = ''.join(ab[x['name']] if x['rect'] == x['full'] else ab[x['name']].lower() for x in b)
    return (not d and not cl), len(full), len(t), s
if __name__ == '__main__':
    cols, rows = int(sys.argv[1]), int(sys.argv[2])
    out = []
    for a in range(-10, 90):
        for b in range(-3, 20):
            ox, oy = 7.0 * a, 22.0 * b
            z8 = ev(cols, rows, ox, oy, 1, True)
            if not z8[0] or z8[1] < 2:
                continue
            u1 = ev(cols, rows, ox, oy, 0, False)
            f1 = ev(cols, rows, ox, oy, 0, True)
            out.append((z8[1] + z8[2] * 0.1 + u1[0] + f1[0], a, b, 'z0.8', z8, 'unf1', u1, 'fold1', f1))
    out.sort(reverse=True)
    for o in out[:40]:
        print(o)
