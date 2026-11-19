import subprocess,sys,re
old,new=sys.argv[1],sys.argv[2]
files=sys.argv[3:]
def run(b,f):
    o=subprocess.run([b,f],capture_output=True,text=True).stdout
    m=re.search(r'n=(\d+) canvas (\d+)x(\d+) boxfill ([\d.]+)%.*rel \(([\d.]+),([\d.]+)\) trunks (\d+)-(\d+) spokes\(all\) (\d+)-(\d+) ratio ([\d.]+) time ([\d.]+)ms',o)
    return m.groups() if m else None
print(f"{'file':14} {'n':>4} | {'fill':>5} {'ratio':>6} {'maxsp':>6} {'ms':>6} {'canvas':>10} | {'fill':>5} {'ratio':>6} {'maxsp':>6} {'ms':>6} {'canvas':>10}")
for f in files:
    a=run(old,f); b=run(new,f)
    if not a or not b: print(f, 'ERR'); continue
    print(f"{f.split('/')[-1][:14]:14} {a[0]:>4} | {a[3]:>5} {a[10]:>6} {a[9]:>6} {a[11]:>6} {a[1]+'x'+a[2]:>10} | {b[3]:>5} {b[10]:>6} {b[9]:>6} {b[11]:>6} {b[1]+'x'+b[2]:>10}")
