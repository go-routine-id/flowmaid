import os, re, sys, glob, shutil
tree, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
n = 0
for p in glob.glob(f"{tree}/examples/*.mmd") + glob.glob(f"{tree}/tests/fixtures/**/*.mmd", recursive=True):
    rel = os.path.relpath(p, tree).replace('/', '__')
    shutil.copy(p, f"{out}/{rel}"); n += 1
readme = open(f"{tree}/README.md").read()
for i, m in enumerate(re.finditer(r"```mermaid\n(.*?)```", readme, re.S)):
    open(f"{out}/readme_{i:03d}.mmd", "w").write(m.group(1)); n += 1
def w(name, s):
    open(f"{out}/{name}.mmd", "w").write(s)
# flat fan grids: 20 sources x k targets
for total in (399, 400, 401, 420):
    s = "flowchart TD\n"; c = 0
    for a in range(20):
        for b in range(21):
            if c == total: break
            s += f"s{a} --> t{b}\n"; c += 1
    w(f"grid_{total}", s)
# chain-ish random dag deterministic
import random
for total in (400, 401):
    r = random.Random(7); s = "flowchart LR\n"
    for i in range(total):
        a = r.randrange(0, 150); b = a + 1 + r.randrange(0, 4)
        s += f"n{a} --> n{b}\n"
    w(f"dag_{total}", s)
print(n)
