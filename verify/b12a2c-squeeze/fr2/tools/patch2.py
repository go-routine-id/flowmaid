import sys
p=sys.argv[1]; s=open(p).read()
old='    out.push_str(&format!("  edge picks: {tot} tried, {miss} missed\\n"));\n'
assert old in s, "missing"
s=s.replace(old, old+'    let pm0 = pick_painted(name, &sc, "scene", out);\n    out.push_str(&format!("  painted picks missed (scene): {pm0}\\n"));\n    if pm0 > 0 { probs.push(format!("{name} blocks={blocks}: {pm0} painted picks missed")); }\n',1)
open(p,'w').write(s)
