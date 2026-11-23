import json, os, subprocess, sys
W = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-flowcli"
C = f"{W}/caps"; H = f"{W}/html"; P = f"{W}/png"
os.makedirs(H, exist_ok=True); os.makedirs(P, exist_ok=True)
states = json.load(open(f"{C}/states.json"))
spec = []
for s in states:
    i = s["id"]
    spec.append(dict(left=f"{C}/{i:03d}_l.ansi", right=f"{C}/{i:03d}_r.ansi",
                     out=f"{H}/{i:03d}.html", cursor=s["cursor"], pointer=s["pointer"]))
# poster: last state without pointer
last = states[-1]["id"]
spec.append(dict(left=f"{C}/{last:03d}_l.ansi", right=f"{C}/{last:03d}_r.ansi",
                 out=f"{H}/poster.html", cursor=None, pointer=None))
json.dump(spec, open(f"{W}/spec.json", "w"))
out = subprocess.run(["python3", f"{W}/compose.py", f"{W}/spec.json"], check=True, capture_output=True, text=True).stdout
dims = json.loads(out); print(dims)
lst = [[x["out"], x["out"].replace("/html/", "/png/").replace(".html", ".png")] for x in spec]
json.dump(lst, open(f"{W}/list.json", "w"))
subprocess.run(["node", f"{W}/shoot.js", f"{W}/list.json", str(dims["W"]), str(dims["H"]), "2"], check=True, cwd=W)
