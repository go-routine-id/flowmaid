import sys
sys.path.insert(0, '.')
import sim
def show(cols, rows, ox, oy, keys, folded=True, quiet=False):
    v = sim.View(); v.page_rows = rows - 9
    v.ox, v.oy = ox, oy
    for k in keys: v.key(k)
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a)
    d = sim.defects(b, t, a)
    cl = [x['text'] for x in t if x['drawn'] != x['text']]
    print('keys=%r folded=%s zoom=%.4f ox=%.2f oy=%.2f defects=%s clipped=%s' % (keys, folded, v.zoom, v.ox, v.oy, d[0], cl))
    if quiet: return
    for i, l in enumerate(sim.render_text(b, t, cols, rows, a)):
        if a[1] <= i < a[1] + a[3]:
            print('%3d|%s|' % (i, l[a[0]:a[0]+a[2]]))
if __name__ == '__main__':
    show(int(sys.argv[1]), int(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4]), sys.argv[5] if len(sys.argv) > 5 else '', len(sys.argv) < 7)
