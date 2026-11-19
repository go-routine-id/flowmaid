import sys, json
sys.argv_keep = sys.argv
from sift import *
path = sys.argv[1]
d = load(path); m = Model(d)
m.layer_len = [0]*m.nl
for n in m.nodes: m.layer_len[n["l"]] += 1
names = json.load(open(sys.argv[2])) if len(sys.argv) > 2 else None
st = m.from_pos()
def uname(p, u):
    return f"{u[0]}{u[1]}"
best = m.cost(st)
print("start", best)
for p in sorted(st): print(" level", p, [uname(p,u) for u in st[p]])
improved = True
while improved:
    improved = False
    for p in sorted(st):
        seq = st[p]
        for u in list(seq):
            i0 = seq.index(u); seq.pop(i0)
            bi, bc = i0, best
            for i in range(len(seq)+1):
                if i == i0: continue
                seq.insert(i, u); c = m.cost(st); seq.pop(i)
                if c < bc: bc, bi = c, i
            seq.insert(bi, u)
            if bc < best:
                print(" move", p, uname(p,u), i0, "->", bi, "cost", best, "->", bc)
                best = bc; improved = True
for p in sorted(st): print(" level", p, [uname(p,u) for u in st[p]])
