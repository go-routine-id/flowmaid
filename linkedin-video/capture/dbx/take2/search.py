import sys
sys.path.insert(0, '.')
import sim

def state(n, kx, ky):
    v = sim.View()
    for _ in range(abs(n)):
        v.key('-' if n > 0 else '+')
    for _ in range(abs(kx)):
        v.key('l' if kx > 0 else 'h')
    for _ in range(abs(ky)):
        v.key('j' if ky > 0 else 'k')
    return v

def evaluate(cols, rows, v, folded=True):
    a = sim.area_for(cols, rows, folded)
    b, t = sim.paint(v, a)
    d, w = sim.defects(b, t, a)
    full = [x['name'] for x in b if x['rect'] == x['full']]
    cutv = [x['name'] for x in b if x['rect'] != x['full'] and x['rect'][0] == x['full'][0] and x['rect'][2] == x['full'][2]]
    cuth = [x['name'] for x in b if x['rect'][0] != x['full'][0] or x['rect'][2] != x['full'][2]]
    toks = [x['text'] for x in t if x['drawn'] == x['text']]
    clipped = [x['text'] for x in t if x['drawn'] != x['text']]
    return dict(d=d, w=w, full=full, cutv=cutv, cuth=cuth, toks=toks, clipped=clipped)

if __name__ == '__main__':
    cols, rows, n = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    KX = range(-12, 34)
    KY = range(-6, 30)
    print('cols %d rows %d zoom-presses %d   legend: digit=#full boxes (clean), c=clean but h-cut box, .=defect' % (cols, rows, n))
    print('     ' + ''.join('%-4d' % k if k % 4 == 0 else '' for k in KX))
    for ky in KY:
        line = ''
        for kx in KX:
            e = evaluate(cols, rows, state(n, kx, ky))
            if e['d']:
                line += '.'
            elif e['cuth']:
                line += 'c'
            else:
                line += str(min(len(e['full']), 9)) if (e['toks']) else chr(ord('a') + min(len(e['full']), 9)).upper()
        print('%4d %s' % (ky, line))
