#!/usr/bin/env python3
"""Writes the hand-checked selftest drawings in both SVG dialects.

The geometry below is simple enough to measure with a ruler; the expected
numbers in expected.json were worked out BY HAND from it (see README.md,
"Validation"), not produced by blockmetrics.
"""
import json, os

HERE = os.path.dirname(os.path.abspath(__file__))

CASES = {
    # TD. Three sibling frames (F1 | F2 side by side, F3 stacked under them),
    # one crossing (e1 x e2), one run through a foreign frame (e3 through F2),
    # a label on a foreign node, two overlapping chips, one struck chip.
    "hand": dict(
        W=600, H=400, dir="TD",
        frames=[("F1", None, 20, 20, 200, 100), ("F2", None, 300, 20, 200, 100), ("F3", None, 20, 220, 480, 120)],
        nodes=[("A", "F1", 40, 50, 80, 40), ("B", "F1", 130, 50, 80, 40), ("C", "F2", 350, 50, 100, 40),
               ("D", "F3", 60, 270, 80, 40), ("E", "F3", 380, 270, 80, 40), ("T", None, 540, 150, 40, 40)],
        # (from, to, points, label (text, cx, cy, w, h) | None, hop at point index | None)
        edges=[("A", "D", [(80, 90), (80, 270)], ("one", 80, 180, 40, 20)),
               ("C", "D", [(400, 90), (400, 165), (70, 165), (70, 270)], ("two", 105, 165, 50, 20)),
               ("B", "E", [(170, 50), (170, 35), (520, 35), (520, 290), (460, 290)], ("three", 520, 170, 60, 20)),
               ("D", "E", [(140, 290), (380, 290)], None),
               ("A", "B", [(120, 70), (130, 70)], ("five", 125, 70, 30, 20))],
        # mermaid dialect only: draw e2 with a hop arc over e1 at (80, 165)
        hops={1: "M400,90L400,165L86,165A6,6 0 0 0 74,165L70,165L70,270"},
        # mermaid dialect only: these ids live in a nested <g class="root" transform=...>
        nested=("F3", 20, 220),
    ),
    # LR. A nest (P holds K), an edge that ends on a frame (S -> P), an edge
    # through the foreign nest (U -> R1: one run, counted at P only), a
    # staggered pair (Q, R), a node hit (V).
    "hand2": dict(
        W=500, H=300, dir="LR",
        frames=[("P", None, 20, 20, 200, 200), ("Q", None, 260, 100, 100, 150), ("R", None, 300, 10, 150, 70), ("K", "P", 40, 60, 100, 80)],
        nodes=[("K1", "K", 60, 80, 60, 40), ("P1", "P", 150, 160, 50, 40), ("Q1", "Q", 280, 140, 60, 40),
               ("R1", "R", 350, 25, 60, 40), ("S", None, 440, 200, 40, 40), ("U", None, 100, 250, 60, 40), ("V", None, 240, 30, 40, 30)],
        edges=[("U", "R1", [(130, 250), (130, 45), (350, 45)], None),
               ("K1", "P1", [(90, 120), (90, 180), (150, 180)], None),
               ("Q1", "S", [(340, 160), (460, 160), (460, 200)], None),
               ("S", "P", [(440, 220), (400, 220), (400, 190), (220, 190)], ("to P", 350, 190, 40, 20))],
        hops={}, nested=None,
    ),
    # TD. Two frames side by side WITH an edge between them (a -> b), a third
    # stacked under both and linked to one of them (b -> c).
    "hand3": dict(
        W=320, H=260, dir="TD",
        frames=[("A1", None, 20, 20, 100, 80), ("B1", None, 200, 20, 100, 80), ("C1", None, 20, 160, 280, 80)],
        nodes=[("a", "A1", 40, 50, 60, 30), ("b", "B1", 220, 50, 60, 30), ("c", "C1", 130, 190, 60, 30)],
        edges=[("a", "b", [(100, 65), (220, 65)], None),
               ("b", "c", [(250, 80), (250, 130), (160, 130), (160, 190)], None)],
        hops={}, nested=None,
    ),
}


def fm_svg(c):
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{c["W"]}" height="{c["H"]}" viewBox="0 0 {c["W"]} {c["H"]}" font-family="Helvetica, Arial, sans-serif" font-size="14" role="img" aria-label="Flowchart diagram">',
         '<title>Flowchart diagram</title>',
         f'<rect width="{c["W"]}" height="{c["H"]}" fill="#ffffff"/>',
         '<defs><marker id="arrow" viewBox="0 0 10 10" refX="5" refY="5" markerUnits="userSpaceOnUse" markerWidth="11" markerHeight="11" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z" fill="#44507a"/></marker>'
         '<mask id="m0" maskUnits="userSpaceOnUse" x="0" y="0" width="10" height="10"><rect x="0" y="0" width="10" height="10" fill="#fff"/><circle cx="5" cy="5" r="5.00" fill="#000"/></mask></defs>']
    titles, chips = [], []
    for (fid, _parent, x, y, w, h) in c["frames"]:
        o.append(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" rx="8" fill="#f6f7fb" stroke="#858db0" stroke-width="1.4"/>')
        titles.append(f'<rect x="{x + 6:.1f}" y="{y + 5:.1f}" width="30.0" height="16" rx="4" fill="#f6f7fb" fill-opacity="0.9"/>\n'
                      f'<text x="{x + 10:.1f}" y="{y + 17:.1f}" font-size="11" font-weight="bold" fill="#4a5378">{fid}</text>')
    for (a, b, pts, lab) in c["edges"]:
        d = " ".join(("M" if i == 0 else "L") + f" {x:.1f} {y:.1f}" for i, (x, y) in enumerate(pts))
        t = f"{a} → {b}" + (f": {lab[0]}" if lab else "")
        o.append(f'<path d="{d}" fill="none" stroke="#44507a" stroke-width="2" marker-end="url(#arrow)"><title>{t}</title></path>')
        if lab:
            text, cx, cy, w, h = lab
            chips.append(f'<rect x="{cx - w / 2:.1f}" y="{cy - 10:.1f}" width="{w:.1f}" height="20" rx="4" fill="#ffffff" stroke="#d5d9ec"/>\n'
                         f'<text x="{cx:.1f}" y="{cy:.1f}" dy="0.33em" text-anchor="middle" fill="#232840">{text}</text>')
    for (nid, _f, x, y, w, h) in c["nodes"]:
        o.append(f'<g><title>{nid}</title>\n<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" rx="5.0" fill="#eef0fb" stroke="#5b66b0" stroke-width="1.8"/>\n'
                 f'<text x="{x + w / 2:.1f}" y="{y + h / 2:.1f}" dy="0.33em" text-anchor="middle" fill="#232840">{nid}</text>\n</g>')
    return "\n".join(o + chips + titles) + "\n</svg>\n"


def mermaid_svg(c):
    sid = "t1"
    nested = c["nested"]
    def inside(fid_or_node_frame):
        return nested is not None and fid_or_node_frame == nested[0]
    def group(items):
        return "".join(items)
    def parts(sel, ox, oy):
        """clusters / edges / labels / nodes whose home is `sel` (True = nested root), shifted by (-ox, -oy)"""
        cl, ed, lb, nd = [], [], [], []
        for (fid, _p, x, y, w, h) in c["frames"]:
            if inside(fid) != sel:
                continue
            cl.append(f'<g class="cluster" id="{sid}-{fid}" data-look="neo"><rect style="" x="{x - ox}" y="{y - oy}" width="{w}" height="{h}"></rect>'
                      f'<g class="cluster-label" transform="translate({x - ox + 4}, {y - oy})"><foreignObject width="30" height="21"><div xmlns="http://www.w3.org/1999/xhtml"><span class="nodeLabel"><p>{fid}</p></span></div></foreignObject></g></g>')
        frame_of = {n[0]: n[1] for n in c["nodes"]}
        for i, (a, b, pts, lab) in enumerate(c["edges"]):
            home = inside(frame_of.get(a)) and inside(frame_of.get(b))
            if home != sel:
                continue
            if i in c["hops"] and not sel:
                d = c["hops"][i]
            else:
                d = "".join(("M" if k == 0 else "L") + f"{x - ox},{y - oy}" for k, (x, y) in enumerate(pts))
            eid = f"L_{a}_{b}_0"
            ed.append(f'<path d="{d}" id="{sid}-{eid}" class="edge-thickness-normal edge-pattern-solid flowchart-link" style="" data-edge="true" data-et="edge" data-id="{eid}"></path>')
            if lab:
                text, cx, cy, w, h = lab
                lb.append(f'<g class="edgeLabel" transform="translate({cx - ox}, {cy - oy})"><g class="label" data-id="{eid}" transform="translate({-w / 2}, {-h / 2})">'
                          f'<foreignObject width="{w}" height="{h}"><div xmlns="http://www.w3.org/1999/xhtml" class="labelBkg"><span class="edgeLabel"><p>{text}</p></span></div></foreignObject></g></g>')
        for k, (nid, f, x, y, w, h) in enumerate(c["nodes"]):
            if inside(f) != sel:
                continue
            nd.append(f'<g class="node default" id="{sid}-flowchart-{nid}-{k}" data-look="neo" transform="translate({x + w / 2 - ox}, {y + h / 2 - oy})">'
                      f'<rect class="basic label-container" style="" x="{-w / 2}" y="{-h / 2}" width="{w}" height="{h}"></rect>'
                      f'<g class="label" style="" transform="translate(-10, -10.5)"><rect></rect><foreignObject width="20" height="21"><div xmlns="http://www.w3.org/1999/xhtml"><span class="nodeLabel"><p>{nid}</p></span></div></foreignObject></g></g>')
        return cl, ed, lb, nd
    cl, ed, lb, nd = parts(False, 0, 0)
    inner = ""
    if nested:
        icl, ied, ilb, ind = parts(True, nested[1], nested[2])
        inner = (f'<g class="root" transform="translate({nested[1]}, {nested[2]})"><g class="clusters">{group(icl)}</g><g class="edges edgePaths">{group(ied)}</g>'
                 f'<g class="edgeLabels">{group(ilb)}</g><g class="nodes">{group(ind)}</g></g>')
    return (f'<svg id="{sid}" width="100%" xmlns="http://www.w3.org/2000/svg" class="flowchart" style="max-width: {c["W"]}px;" viewBox="0 0 {c["W"]} {c["H"]}" role="graphics-document document" aria-roledescription="flowchart-v2">'
            f'<style>#{sid}{{font-family:sans-serif;}} #{sid} .node rect{{fill:#eee;}}</style><g>'
            f'<marker id="{sid}_flowchart-v2-pointEnd" class="marker flowchart-v2" viewBox="0 0 10 10" refX="5" refY="5" markerUnits="userSpaceOnUse" markerWidth="8" markerHeight="8" orient="auto"><path d="M 0 0 L 10 5 L 0 10 z" class="arrowMarkerPath"></path></marker>'
            f'<g class="root"><g class="clusters">{group(cl)}</g><g class="edges edgePaths">{group(ed)}</g><g class="edgeLabels">{group(lb)}</g><g class="nodes">{group(nd)}{inner}</g></g></g></svg>\n')


def struct_json(c):
    fidx = {f[0]: i for i, f in enumerate(c["frames"])}
    nidx = {n[0]: i for i, n in enumerate(c["nodes"])}
    def end(x):
        return {"n": nidx[x]} if x in nidx else {"s": fidx[x]}
    node_edges = [e for e in c["edges"] if e[0] in nidx and e[1] in nidx]
    sub_edges = [e for e in c["edges"] if not (e[0] in nidx and e[1] in nidx)]
    return json.dumps({
        "tool": "hand-written", "kind": "flowchart", "direction": c["dir"],
        "nodes": [{"id": n[0], "label": n[0], "shape": "Rect", "sub": fidx[n[1]] if n[1] else None} for n in c["nodes"]],
        "subgraphs": [{"id": f[0], "title": f[0], "parent": fidx[f[1]] if f[1] else None, "direction": None,
                       "nodes": [nidx[n[0]] for n in c["nodes"] if n[1] == f[0]]} for f in c["frames"]],
        "edges": [{"from": end(e[0]), "to": end(e[1]), "label": e[3][0] if e[3] else None, "kind": "Arrow", "drawn": True} for e in node_edges + sub_edges],
    }, indent=1) + "\n"


for name, c in CASES.items():
    # flowmaid paints Graph::edges first, then the edges that end on a subgraph
    nid = {n[0] for n in c["nodes"]}
    c["edges"] = [e for e in c["edges"] if e[0] in nid and e[1] in nid] + [e for e in c["edges"] if not (e[0] in nid and e[1] in nid)]
    open(os.path.join(HERE, f"{name}-fm.svg"), "w").write(fm_svg(c))
    open(os.path.join(HERE, f"{name}-mermaid.svg"), "w").write(mermaid_svg(c))
    open(os.path.join(HERE, f"{name}.struct.json"), "w").write(struct_json(c))
print("wrote", ", ".join(sorted(f for f in os.listdir(HERE) if f.endswith((".svg", ".struct.json")))))
