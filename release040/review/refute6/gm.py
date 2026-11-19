import sys, math, statistics as st
d = sys.argv[1]
def load(f):
    rows = {}
    for i, l in enumerate(open(f)):
        if i == 0: continue
        p = l.rstrip('\n').split('\t')
        rows[p[0]] = (float(p[2]), float(p[3]), int(p[4]))
    return rows
A = [load(f'{d}/a{i}.tsv') for i in (1,2,3)]
B = [load(f'{d}/b{i}.tsv') for i in (1,2,3)]
names = sorted(A[0])
def gm(xs): return math.exp(sum(math.log(x) for x in xs)/len(xs))
for i in range(3):
    print('round', i+1, 'median gm', round(gm([B[i][n][0]/A[i][n][0] for n in names]),4), 'min gm', round(gm([B[i][n][1]/A[i][n][1] for n in names]),4))
print('fastest of three medians', round(gm([min(b[n][0] for b in B)/min(a[n][0] for a in A) for n in names]),4))
print('fastest of three mins', round(gm([min(b[n][1] for b in B)/min(a[n][1] for a in A) for n in names]),4))
print('median of three medians', round(gm([st.median(b[n][0] for b in B)/st.median(a[n][0] for a in A) for n in names]),4))
print('mean of three medians', round(gm([st.mean(b[n][0] for b in B)/st.mean(a[n][0] for a in A) for n in names]),4))
print('all nine pairs pooled', round(gm([b[n][0]/a[n][0] for n in names for a in A for b in B]),4))
print('3 matched pairs pooled', round(gm([B[i][n][0]/A[i][n][0] for n in names for i in range(3)]),4))
print('svg bytes equal', all(A[i][n][2]==B[j][n][2] for n in names for i in range(3) for j in range(3)))
for n in names:
    print(n, [A[i][n][0] for i in range(3)], [B[i][n][0] for i in range(3)])
