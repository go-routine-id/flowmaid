#!/usr/bin/env python3
"""Compare base/head SVG pairs kept by the probe.

For each `diff` row: the head viewBox origin (ox, oy) must be negative on
some axis and head must be base translated by (ox, oy): every x coordinate
differs by ox, every y by oy (to print rounding), everything else equal.
Usage: cmp.py <head_out_dir> <base_out_dir> [--show N]
"""
import re
import sys
from collections import Counter, defaultdict

TAG = re.compile(r"<[^>]*>|[^<]+")
ATTR = re.compile(r'([\w:-]+)="([^"]*)"')
NUM = re.compile(r"-?\d+(?:\.\d+)?(?:e[-+]?\d+)?")
XA = {"x", "x1", "x2", "cx", "fx"}
YA = {"y", "y1", "y2", "cy", "fy"}
TOL = 0.151


def parse_vb(svg):
    m = re.search(r'<svg[^>]* viewBox="([^"]*)"', svg)
    return [float(v) for v in m.group(1).split()]


def path_axes(d):
    """(value, axis) for every number of a path's d; axis 'x', 'y' or '0'."""
    out = []
    toks = re.findall(r"[A-Za-z]|" + NUM.pattern, d)
    cmd = None
    k = 0
    for t in toks:
        if t.isalpha():
            cmd = t
            k = 0
            continue
        v = float(t)
        if cmd is None:
            out.append((v, "?"))
            continue
        if cmd.islower():
            out.append((v, "0"))
        elif cmd in "MLCSQT":
            out.append((v, "x" if k % 2 == 0 else "y"))
        elif cmd == "H":
            out.append((v, "x"))
        elif cmd == "V":
            out.append((v, "y"))
        elif cmd == "A":
            j = k % 7
            out.append((v, "x" if j == 5 else "y" if j == 6 else "0"))
        else:
            out.append((v, "?"))
        k += 1
    return out


def skeleton(s):
    return NUM.sub("#", s)


def tokens(svg):
    """List of (kind, skeleton, [(value, axis)]) per tag / text chunk."""
    out = []
    in_marker = 0
    for t in TAG.findall(svg):
        if not t.startswith("<"):
            out.append(("text", t, []))
            continue
        name = re.match(r"</?([\w:-]+)", t)
        name = name.group(1) if name else "?"
        if t.startswith("</"):
            if name in ("marker", "pattern"):
                in_marker -= 1
            out.append(("tag", t, []))
            continue
        nums = []
        if name == "svg":
            for a, v in ATTR.findall(t):
                if a == "viewBox":
                    vs = [float(x) for x in v.split()]
                    nums += [(vs[0], "x"), (vs[1], "y"), (vs[2], "0"), (vs[3], "0")]
                else:
                    nums += [(float(n), "0") for n in NUM.findall(v)]
        else:
            for a, v in ATTR.findall(t):
                if in_marker:
                    nums += [(float(n), "0") for n in NUM.findall(v)]
                elif a in XA:
                    nums += [(float(n), "x") for n in NUM.findall(v)]
                elif a in YA:
                    nums += [(float(n), "y") for n in NUM.findall(v)]
                elif a == "d":
                    nums += path_axes(v)
                elif a == "points":
                    ns = NUM.findall(v)
                    nums += [(float(n), "x" if i % 2 == 0 else "y") for i, n in enumerate(ns)]
                elif a == "transform":
                    for fn, args in re.findall(r"(\w+)\(([^)]*)\)", v):
                        ns = NUM.findall(args)
                        if fn == "translate":
                            nums += [(float(n), "x" if i == 0 else "y") for i, n in enumerate(ns)]
                        elif fn == "rotate":
                            nums += [(float(n), ["0", "x", "y"][i]) for i, n in enumerate(ns)]
                        else:
                            nums += [(float(n), "?") for n in ns]
                else:
                    nums += [(float(n), "0") for n in NUM.findall(v)]
        if name in ("marker", "pattern") and not t.endswith("/>"):
            in_marker += 1
        out.append(("tag", skeleton(t), nums))
    return out


def strip_bg(svg, ox, oy):
    """Head writes the background rect at the canvas corner; base had none."""
    return re.sub(r'<rect x="[^"]*" y="[^"]*" (width="[^"]*" height="[^"]*" fill=)', r"<rect \1", svg, count=1)


def compare(base, head):
    vb = parse_vb(head)
    bvb = parse_vb(base)
    ox, oy = vb[0], vb[1]
    res = {"ox": ox, "oy": oy, "bvb": bvb[:2]}
    if ox == 0 and oy == 0:
        res["verdict"] = "VB00_DIFF"
        return res
    if ox > 0 or oy > 0:
        res["verdict"] = "POSITIVE_ORIGIN"
        return res
    head2 = strip_bg(head, ox, oy)
    bt, ht = tokens(base), tokens(head2)
    if len(bt) != len(ht):
        res["verdict"] = "STRUCTURE"
        res["detail"] = f"{len(bt)} vs {len(ht)} tokens"
        return res
    bad = []
    for i, (b, h) in enumerate(zip(bt, ht)):
        if b[0] != h[0] or b[1] != h[1] or len(b[2]) != len(h[2]):
            bad.append(("skel", i, b[1][:160], h[1][:160]))
            continue
        for (bv, ax), (hv, ax2) in zip(b[2], h[2]):
            want = ox if ax == "x" else oy if ax == "y" else 0.0
            if ax == "?":
                bad.append(("axis?", i, b[1][:120], bv, hv))
            elif abs((hv - bv) - want) > TOL:
                bad.append(("num", i, b[1][:200], ax, bv, hv, want))
    res["verdict"] = "TRANSLATION" if not bad else "MISMATCH"
    res["bad"] = bad
    return res


def main():
    hd, bd = sys.argv[1], sys.argv[2]
    show = int(sys.argv[sys.argv.index("--show") + 1]) if "--show" in sys.argv else 0
    files = {}
    rows = []
    vout = open(sys.argv[sys.argv.index("--verdicts") + 1], "w") if "--verdicts" in sys.argv else None
    for line in open(f"{hd}/results.tsv"):
        f = line.rstrip("\n").split("\t")
        if f[1] == "file":
            files[f[0]] = f[4]
            continue
        rows.append(f)
    tally = Counter()
    by = defaultdict(list)
    for f in rows:
        fid, mode, _, status = f[:4]
        if status != "diff":
            tally[(mode.split(".")[0] if "." in mode else mode, "same")] += 1
            continue
        name = f"{fid}.{mode}.svg"
        base = open(f"{bd}/{name}").read()
        head = open(f"{hd}/{name}").read()
        if mode == "render" or mode == "scene":
            r = {"verdict": "STATIC_DIFF"}
        elif base.startswith("PANIC") or head.startswith("PANIC"):
            r = {"verdict": "PANIC"}
        else:
            r = compare(base, head)
        tally[(mode, r["verdict"])] += 1
        by[r["verdict"]].append((files[fid], mode, r))
        if vout:
            vout.write(f"{fid}\t{mode}\t{r['verdict']}\t{files[fid]}\n")
    for k in sorted(tally):
        print(*k, tally[k])
    for v, items in by.items():
        if v == "TRANSLATION":
            continue
        print(f"== {v}: {len(items)}")
        for path, mode, r in items[: show or 10]:
            print("  ", path, mode, {k: r[k] for k in r if k != "bad"})
            for b in r.get("bad", [])[:6]:
                print("      ", b)


if __name__ == "__main__":
    main()
