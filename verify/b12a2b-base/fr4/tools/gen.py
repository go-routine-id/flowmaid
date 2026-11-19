import sys, os
out = sys.argv[1]
titles = {
 "w": "WWW Mammoth Wombat Warehouse Overview",
 "s": "Shipping and returns desk",
 "n": "Édition générale des données",
 "m": "MMMM WWWW",
}
shapes = {}
shapes["pins"] = """flowchart TD
    s1[s1] --> G
    s2[s2] --> G
    s3[s3] --> G
    s4[s4] --> x
    subgraph G[{T}]
        x[member x]
        y[y]
        x --> y
    end
    y --> z[end]
"""
shapes["mix"] = """flowchart TD
    a[alpha] --> x
    b[beta] --> G
    c[gamma] --> y
    d[delta] --> G
    d --> x
    e[eps] --> y
    subgraph G[{T}]
        x[x node wide one]
        y[y]
        w[w]
        x --> w
        y --> w
    end
    w --> out[out]
"""
shapes["nested"] = """flowchart TD
    a[a] --> x
    b[b] --> I
    c[c] --> O
    d[d] --> y
    subgraph O[{T}]
        subgraph I[{T} inner]
            x[x wide member]
            y[y]
        end
        k[k]
    end
    x --> k
    y --> k
    k --> z[z]
"""
shapes["exits"] = """flowchart TD
    subgraph G[{T}]
        x[x member that is long]
        y[y]
        x --> y
    end
    s[src] --> x
    x --> o1[o1]
    x --> o2[o2]
    y --> o3[o3]
    o1 --> s
"""
shapes["comb"] = """flowchart TD
    h[hub] --> x1
    h --> x2
    h --> x3
    h --> x4
    k[other] --> x4
    subgraph G[{T}]
        x1[x1]
        x2[x2]
        x3[x3]
        x4[x4]
    end
    x1 --> z[z]
    x3 --> z
"""
shapes["twin"] = """flowchart TD
    subgraph A[{T}]
        a1[a1 wide node]
        a2[a2]
    end
    subgraph B[{T} two]
        b1[b1]
        b2[b2 wide node]
    end
    a1 --> b1
    a2 --> b2
    a1 --> b2
    p[p] --> A
    p --> a2
    q[q] --> B
    q --> b1
"""
shapes["labels"] = """flowchart TD
    s1[s1] -- "first label" --> x
    s2[s2] -- "second" --> x
    s3[s3] --> x
    s4[s4] -- "to frame" --> G
    subgraph G[{T}]
        x[wide member node that is long]
        y[y]
        x --> y
    end
    y --> z[end]
"""
shapes["state"] = """stateDiagram-v2
    direction TB
    [*] --> A
    A --> X
    B --> X
    C --> G
    state "{T}" as G {
        X --> Y
        Y --> [*]
    }
    G --> D
    A --> B
    B --> C
"""
for sn, src in shapes.items():
    for tk, t in titles.items():
        s = src.replace("{T}", t)
        for d in ["TD", "BT", "LR", "RL"]:
            if sn == "state":
                dd = "TB" if d == "TD" else d
                body = s.replace("direction TB", "direction " + dd)
            else:
                body = s.replace("flowchart TD", "flowchart " + d, 1)
            open(os.path.join(out, f"{sn}-{tk}-{d}.mmd"), "w").write(body)
