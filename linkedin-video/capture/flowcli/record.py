#!/usr/bin/env python3
"""Drive the real tmux session (left: bash, right: flowcli --watch) and capture
each visible state with `tmux capture-pane -e -p`. Writes caps/NNN_{l,r}.ansi
and caps/states.json = [{id, frames, cursor, pointer, note}]."""
import json, os, subprocess, time
W = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-flowcli"
C = os.path.join(W, "caps")
os.makedirs(C, exist_ok=True)
for f in os.listdir(C):
    os.remove(os.path.join(C, f))
T = ["tmux", "-L", "flowvid", "-f", os.path.join(W, "tmux.conf")]

def tm(*a):
    return subprocess.run(T + list(a), check=True, capture_output=True, text=True).stdout

def wait(s):
    time.sleep(s)

states = []
def cap(frames, note, cursor=True, pointer=None):
    i = len(states)
    l = tm("capture-pane", "-t", "v.0", "-e", "-p")
    r = tm("capture-pane", "-t", "v.1", "-e", "-p")
    open(f"{C}/{i:03d}_l.ansi", "w").write(l)
    open(f"{C}/{i:03d}_r.ansi", "w").write(r)
    cur = None
    if cursor:
        x, y = tm("display", "-p", "-t", "v.0", "#{cursor_x} #{cursor_y}").split()
        cur = [int(x), int(y)]
    states.append(dict(id=i, frames=frames, cursor=cur, pointer=pointer, note=note))

def mouse(b, x, y, up=False):
    # SGR (1006) mouse report, 1-based cell coordinates, sent as pane input
    tm("send-keys", "-t", "v.1", "-l", "--", f"\x1b[<{b};{x};{y}{'m' if up else 'M'}")

subprocess.run([os.path.join(W, "setup.sh")], check=True)
wait(0.5)
cap(33, "initial: source on the left, flowcli --watch on the right")

cmd = "echo '    D --> N[Email buyer]' >> flow.mmd"
for k in range(0, len(cmd), 3):
    tm("send-keys", "-t", "v.0", "-l", "--", cmd[k:k + 3])
    wait(0.08)
    cap(2, "typing")
tm("send-keys", "-t", "v.0", "Enter")
wait(0.03)
cap(6, "enter pressed (file appended), reload not drawn yet")
wait(0.9)
cap(42, "--watch reloaded: 9 nodes")

X0, Y0, X1, Y1 = 36, 30, 54, 24
mouse(0, X0, Y0)
wait(0.25)
cap(6, "press on Backorder: picked, focus D", cursor=False, pointer=[X0 - 1, Y0 - 1])
N = 10
for i in range(1, N + 1):
    x = X0 + round((X1 - X0) * i / N)
    y = Y0 + round((Y1 - Y0) * i / N)
    mouse(32, x, y)
    wait(0.2)
    cap(3, f"drag step {i}", cursor=False, pointer=[x - 1, y - 1])
mouse(0, X1, Y1, up=True)
wait(0.25)
cap(27, "released: edges re-routed", cursor=False, pointer=[X1 - 1, Y1 - 1])
mouse(0, 56, 8)
wait(0.05)
mouse(0, 56, 8, up=True)
wait(0.3)
cap(30, "click on empty canvas: selection cleared", cursor=False, pointer=[55, 7])
json.dump(states, open(f"{C}/states.json", "w"), indent=1)
print(len(states), "states,", sum(s["frames"] for s in states), "frames")
