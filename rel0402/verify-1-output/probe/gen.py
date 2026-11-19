"""Generate random inputs of the #112 scene types (and ER/class/mindmap) for picking/drag fuzz."""
import random, sys, os

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
R = random.Random(20261118)


def w():
    return R.choice(["alpha", "beta", "gamma", "delta", "Long label here", "x", "Customer Order", "日本語", "a&b", "<b>bold</b>"])


def seq(i):
    n = R.randint(1, 8)
    parts = [f"P{k}" for k in range(n)]
    s = ["sequenceDiagram"]
    if R.random() < 0.4:
        s.append("autonumber")
    for p in parts:
        kind = "actor" if R.random() < 0.3 else "participant"
        s.append(f"{kind} {p} as {w()}")
    fr = []
    for _ in range(R.randint(1, 40)):
        r = R.random()
        a, b = R.choice(parts), R.choice(parts)
        arrow = R.choice(["->>", "-->>", "->", "-->", "-x", "--x", "-)", "--)"])
        if r < 0.45:
            s.append(f"{a}{arrow}{b}: {w()}")
        elif r < 0.55:
            s.append(f"Note {R.choice(['left of','right of','over'])} {a}: {w()}")
        elif r < 0.6 and n > 1:
            s.append(f"Note over {parts[0]},{parts[-1]}: {w()}")
        elif r < 0.7 and len(fr) < 4:
            k = R.choice(['loop','alt','opt','par'])
            fr.append(k)
            s.append(f"{k} {w()}")
        elif r < 0.75 and fr and fr[-1] in ('alt', 'par'):
            s.append(("else " if fr[-1] == 'alt' else "and ") + w())
        elif r < 0.85 and fr:
            s.append("end")
            fr.pop()
        elif r < 0.9:
            s.append(f"activate {a}")
            s.append(f"{a}->>{b}: act")
            s.append(f"deactivate {a}")
        else:
            s.append(f"{a}->>{a}: self {w()}")
    s += ["end"] * len(fr)
    return "\n".join(s) + "\n"


def pie(i):
    s = ["pie" + (" showData" if R.random() < 0.5 else "")]
    if R.random() < 0.5:
        s.append(f"title {w()}")
    for k in range(R.randint(1, 24)):
        v = R.choice([0, 0.01, 0.4, 1, 5, 10, 100, 1000, R.random() * 50])
        s.append(f'"{w()} {k}" : {v}')
    return "\n".join(s) + "\n"


def journey(i):
    s = ["journey"]
    if R.random() < 0.6:
        s.append(f"title {w()}")
    actors = [f"A{k}" for k in range(R.randint(1, 12))]
    for sec in range(R.randint(1, 5)):
        if R.random() < 0.9:
            s.append(f"section {w()} {sec}")
        for t in range(R.randint(1, 6)):
            who = ", ".join(R.sample(actors, R.randint(1, len(actors))))
            s.append(f"  {w()} {t}: {R.randint(1,5)}: {who}")
    return "\n".join(s) + "\n"


def timeline(i):
    s = ["timeline"]
    if R.random() < 0.6:
        s.append(f"title {w()}")
    for sec in range(R.randint(0, 4)):
        if sec > 0 or R.random() < 0.5:
            s.append(f"section {w()} {sec}")
        for p in range(R.randint(1, 5)):
            evs = " : ".join(w() for _ in range(R.randint(1, 4)))
            s.append(f"  {2000 + p} : {evs}")
    return "\n".join(s) + "\n"


def sankey(i):
    s = ["sankey-beta", ""]
    n = R.randint(2, 30)
    for k in range(R.randint(1, 60)):
        a, b = R.randrange(n), R.randrange(n)
        if a >= b:
            continue
        s.append(f"N{a},N{b},{R.choice([0.1, 1, 5, 20, 100, R.random()*300]):.2f}")
    if len(s) == 2:
        s.append("N0,N1,3")
    return "\n".join(s) + "\n"


def mind(i):
    s = ["mindmap", "  root((Root))"]
    shapes = ["{}", "[{}]", "({})", "(({}))", ")){}((", "){}(", "{{{{{}}}}}"]
    stack = [2]
    for k in range(R.randint(1, 120)):
        d = R.randint(1, min(6, len(stack)))
        stack = stack[:d]
        ind = "  " * (d + 1)
        s.append(ind + R.choice(shapes).format(w() + str(k)))
        if R.random() < 0.1:
            s.append(ind + "  ::icon(fa fa-book)")
        stack.append(d + 1)
    return "\n".join(s) + "\n"


def erd(i):
    n = R.randint(2, 30)
    s = ["erDiagram"]
    if R.random() < 0.3:
        s.append("direction LR")
    cards = ["||--o{", "||--|{", "}o--o{", "|o--||", "}|..|{", "||--||"]
    for k in range(R.randint(1, 50)):
        a, b = R.randrange(n), R.randrange(n)
        s.append(f"E{a} {R.choice(cards)} E{b} : {R.choice(['has','r'+str(k),'\"two words\"'])}")
    for k in range(n):
        if R.random() < 0.5:
            s.append(f"E{k} {{\n  int id PK\n  string name \"comment\"\n}}")
    return "\n".join(s) + "\n"


def cls(i):
    n = R.randint(2, 30)
    s = ["classDiagram"]
    if R.random() < 0.3:
        s.append("direction " + R.choice(["LR", "RL", "BT", "TB"]))
    kinds = ["<|--", "*--", "o--", "-->", "..>", "..|>", "--", "<-->"]
    for k in range(R.randint(1, 50)):
        a, b = R.randrange(n), R.randrange(n)
        lab = f" : {w()}" if R.random() < 0.4 else ""
        if R.random() < 0.3:
            s.append(f'C{a} "1" {R.choice(kinds)} "*" C{b}{lab}')
        else:
            s.append(f"C{a} {R.choice(kinds)} C{b}{lab}")
    for k in range(R.randint(0, 3)):
        if R.random() < 0.5:
            s.append(f'note for C{R.randrange(n)} "{w()}"')
        else:
            s.append(f'note "{w()}"')
    for k in range(n):
        if R.random() < 0.3:
            s.append(f"class C{k} {{\n  +int id\n  +name() String\n}}")
    return "\n".join(s) + "\n"


gens = {"seq": seq, "pie": pie, "journey": journey, "timeline": timeline, "sankey": sankey,
        "mind": mind, "er": erd, "class": cls}
counts = {"seq": 300, "pie": 300, "journey": 200, "timeline": 200, "sankey": 200, "mind": 200, "er": 150, "class": 150}
for k, f in gens.items():
    for i in range(counts[k]):
        open(os.path.join(out, f"g-{k}-{i:03}.mmd"), "w").write(f(i))
print("ok")
