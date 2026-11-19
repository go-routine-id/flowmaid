import json, itertools, sys
names = [
 # plain / dotted names
 "A", "A.B", "a.b.C", "com.example.Order", "A.B.C.D", "A1.B2", "_A._B", "A-B.C-D", "A.-B", "A.1", "1.A", "A.é", "é.A", "A.日本",
 "A._", "A.B-", "A-.B", "A.B_", "A.__", "x.y.z1",
 # stray dots
 "A.", ".A", "A..B", "A...B", "A....B", "A..", "..A", "A.B.", ".A.B", "A..B.C", ".", "A .", " .A", "A . . B", "A-.", "A.B..C",
 # spaces round dots
 "A . B", "A. B", "A .B", "A\t.\tB", "A . B . C",
 # with generics
 "A~T~", "A.B~T~", "A~a.b~", "A.B~a.b~", "A~T~.", "A~T~.B", "A~T~..B", "A.B~T~.C", "A.~T~", "A ~T~", "Map~a.b.C~",
 # backticks
 "`A`", "`A.`", "`A.B`", "`.A`", "A.`B`", "a.b.`C D`", "`A`.", "`A`.B", "`A`.`B`", "`A", "`A.",
 # labels
 'A["lbl"]', 'A.B["lbl"]', 'A.B["x.y"]', 'A["x.y"]', 'A["lbl"].', 'A.["lbl"]', 'A[x]', 'A.B[x.y]', 'A.B ["lbl"]',
 # styles
 "A:::foo", "A.B:::foo", "a.b.C:::foo", "A:::foo.", "A:::a.b", "A.:::foo", 'A.B["lbl"]:::foo', "A.B~T~:::foo",
 # punctuation / misc
 "A.$", "A.B$", "A.;", "A.B;", "A,B", "A.B, C.D", "A., B", "A, B.",
]
suffixes = ["", " {\n}", " {\n+x int\n}", "{\n}", " {}"]
cases = []
for n in names:
    for s in suffixes:
        cases.append(f"classDiagram\nclass {n}{s}")
# annotations
for n in ["A", "A.B", "a.b.C", "A.", ".A", "A..B", "A~T~.", "`A.`", "A.B~T~"]:
    cases.append(f"classDiagram\nclass {n}\n<<interface>> {n}")
    cases.append(f"classDiagram\n<<interface>> {n}")
    cases.append(f"classDiagram\nclass X {{\n<<interface>>\n}}\n<<interface>>{n}")
# relations / members / notes using dotted names (unchanged paths)
for n in ["A.B", "a.b.C", "A.", ".A", "A..B"]:
    cases.append(f"classDiagram\n{n} <|-- C")
    cases.append(f"classDiagram\nC --> {n}")
    cases.append(f"classDiagram\n{n} : +int x")
    cases.append(f"classDiagram\nclass {n}\nnote for {n} \"hi\"")
    cases.append(f"classDiagram\nclass {n}\nstyle {n} fill:#f00")
    cases.append(f"classDiagram\nclass {n}\ncssClass \"{n}\" foo")
    cases.append(f"classDiagram\nclass {n}\nclick {n} href \"http://x\"")
# namespaces
for n in ["A.B", "a.b.C", "A.", "A..B", "A"]:
    cases.append(f"classDiagram\nnamespace com.example {{\nclass {n}\n}}")
    cases.append(f"classDiagram\nnamespace NS {{\nclass {n}\n}}")
# comments and line endings
for n in ["A.B", "A.", "a.b.C"]:
    cases.append(f"classDiagram\n%% c\nclass {n}\n%% d")
    cases.append(f"classDiagram\r\nclass {n}\r\n")
    cases.append(f"classDiagram\nclass {n} %% trailing")
    cases.append(f"classDiagram\n  class {n}  ")
    cases.append(f"classDiagram-v2\nclass {n}")
    cases.append(f"---\ntitle: t\n---\nclassDiagram\nclass {n}")
    cases.append(f"classDiagram\ndirection LR\nclass {n}")
# block with dotted members inside
cases.append("classDiagram\nclass A.B {\n+java.util.List items\n+get() a.b.C\n}")
cases.append("classDiagram\nclass A {\n+x.y\n}")
cases.append("classDiagram\nclass A.B~T~ {\n+T value\n}\nA.B <|-- C.D")
seen = set(); out = []
for c in cases:
    if c in seen: continue
    seen.add(c); out.append(c)
json.dump(out, open(sys.argv[1] + '/cases.json', 'w'))
with open(sys.argv[1] + '/cases.txt', 'w') as f:
    for c in out:
        f.write(c.replace('\\', '\\\\').replace('\n', '\\n').replace('\t', '\\t') + '\n')
print(len(out))
