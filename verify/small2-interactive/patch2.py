p='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/small2-interactive/repo/src/vtest.rs'
s=open(p).read()
old='''    if svg.contains("NaN") || svg.contains("inf") {
        let i = svg.find("NaN").or_else(|| svg.find("inf")).unwrap();
        return Some(format!(
            "svg: ...{}...",
            &svg[i.saturating_sub(40)..(i + 10).min(svg.len())]
        ));
    }
    None'''
new='''    // numeric attributes only (text content may say "information")
    let mut rest = svg;
    while let Some(i) = rest.find("=\\"") {
        let name = rest[..i].rsplit(|c: char| c == ' ' || c == '<').next().unwrap_or("");
        let after = &rest[i + 2..];
        let j = after.find('"').unwrap_or(after.len());
        let val = &after[..j];
        if !matches!(name, "aria-label" | "id" | "class" | "xmlns" | "role" | "font-family" | "text-anchor" | "fill" | "stroke" | "style" | "dominant-baseline" | "aria-labelledby" | "aria-roledescription" | "href" | "marker-end" | "marker-start" | "stroke-linecap" | "stroke-linejoin" | "font-weight" | "font-style" | "clip-path")
        {
            for t in val.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-')) {
                if t.parse::<f64>().is_ok_and(|v| !v.is_finite()) {
                    return Some(format!("svg {name}=\\"{}\\"", &val[..val.len().min(80)]));
                }
            }
        }
        rest = &after[j.min(after.len())..];
    }
    None'''
assert old in s
s=s.replace(old,new)
open(p,'w').write(s)
