import sys
sys.path.insert(0, '.')
import sim
def show(cols, rows, keys, folded=True):
    v = sim.View(); v.page_rows = rows - 9
    for k in keys: v.key(k)
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a)
    print('keys=%r zoom=%.4f ox=%.2f oy=%.2f' % (keys, v.zoom, v.ox, v.oy), sim.defects(b, t, a))
    for i, l in enumerate(sim.render_text(b, t, cols, rows, a)):
        if a[1] <= i < a[1] + a[3]:
            print('%3d|%s|' % (i, l[a[0]:a[0]+a[2]]))
if __name__ == '__main__':
    show(int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], len(sys.argv) < 5)
