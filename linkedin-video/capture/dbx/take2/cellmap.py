import sys
sys.path.insert(0, '.')
import sim
ab = {'categories':'Ca','products':'Pr','order_items':'Oi','customers':'Cu','orders':'Or','payments':'Pa','shipments':'Sh'}
def ev(cols, rows, v, folded=True):
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a); d, w = sim.defects(b, t, a)
    cl = [x for x in t if x['drawn'] != x['text']]
    s = ''.join(ab[x['name']] if x['rect'] == x['full'] else ab[x['name']].lower() for x in b)
    return (not d and not cl), s, len(t)
def at(cols, rows, ox, oy, keys):
    v = sim.View(); v.page_rows = rows - 9; v.ox, v.oy = ox, oy
    for k in keys: v.key(k)
    return v
if __name__ == '__main__':
    cols, rows, a, b = map(int, sys.argv[1:5])
    ox, oy = 7.0 * a, 22.0 * b
    print('drag target unfolded/folded z1 clean:', ev(cols, rows, at(cols, rows, ox, oy, ''), False)[0], ev(cols, rows, at(cols, rows, ox, oy, ''), True)[0])
    for j in range(-1, 6):
        line = ''
        for i in range(-6, 6):
            keys = '-kkk' + ('l' * i if i > 0 else 'h' * -i) + ('j' * j if j > 0 else 'k' * -j)
            c, s, nt = ev(cols, rows, at(cols, rows, ox, oy, keys))
            line += ' %-10s' % ((s + str(nt)) if c else '.')
        print('%3d %s' % (j, line))
    print('    ' + ''.join(' %-10d' % i for i in range(-6, 6)))
