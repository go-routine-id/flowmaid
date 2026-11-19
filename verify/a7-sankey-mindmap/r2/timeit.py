import subprocess, sys, time
bins = sys.argv[1].split(',')
for f in sys.argv[2:]:
    row=[f.split('/')[-1]]
    for b in bins:
        best=1e9
        for _ in range(3):
            t=time.perf_counter(); subprocess.run([b,f],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL); best=min(best,time.perf_counter()-t)
        row.append(f'{best*1000:.0f}ms')
    print(' '.join(row))
