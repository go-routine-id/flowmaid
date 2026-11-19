import re,sys
p='/home/user/flowmaid/src/parser.rs'
s=open(p).read()
start=s.index('/// Read `config.sankey.*` out of YAML frontmatter (see')
end=s.index('/// Apply one `config.sankey.<key>: <value>` pair.')
new='''/// Read `config.sankey.*` out of YAML frontmatter (see
/// [`frontmatter_section`]).
fn sankey_config(source: &str) -> SankeyConfig {
    let mut cfg = SankeyConfig::default();
    // Indent of a `nodeColors:` key whose block map follows. Its entries
    // are node labels, so they must never reach `apply_sankey_key` — a
    // node called `width` would otherwise resize the canvas.
    let mut colors_at: Option<usize> = None;
    for (ind, t) in frontmatter_section(source, "sankey") {
        if let Some(at) = colors_at {
            if ind > at {
                if let Some((k, v)) = yaml_pair(t) {
                    set_node_color(&mut cfg, k, unquote_yaml(v));
                }
                continue;
            }
            colors_at = None;
        }
        if let Some((k, v)) = yaml_pair(t) {
            if k == "nodeColors" {
                let v = v.trim();
                if v.is_empty() {
                    colors_at = Some(ind);
                } else if let Some(map) = v
                    .strip_prefix('{')
                    .and_then(|m| m.trim_end().strip_suffix('}'))
                {
                    for item in yaml_flow_items(map) {
                        if let Some((k, v)) = yaml_pair(item) {
                            set_node_color(&mut cfg, k, unquote_yaml(v));
                        }
                    }
                }
                continue;
            }
            apply_sankey_key(&mut cfg, k, v);
        }
    }
    cfg
}

/// The scalar `key: value` pairs (key trimmed, value raw — run it
/// through [`unquote_yaml`]) of the `config.<section>:` block in YAML
/// frontmatter, in source order; empty when there is none. Nested
/// lines are flattened, so callers see every `key: value` under the
/// section (see [`frontmatter_section`]).
fn frontmatter_config<'a>(source: &'a str, section: &str) -> Vec<(&'a str, &'a str)> {
    frontmatter_section(source, section)
        .into_iter()
        .filter_map(|(_, t)| t.split_once(':').map(|(k, v)| (k.trim(), v)))
        .collect()
}

/// Every line inside the `config.<section>:` block of YAML frontmatter,
/// as (indent, trimmed text) in source order — blank and `#` comment
/// lines left out; empty when there is no such block. The indent lets a
/// caller follow a nested block map (sankey's `nodeColors`).
///
/// Deliberately NOT a YAML parser: it walks indentation for the
/// `config:` -> `<section>:` path and hands back the lines under it.
/// Richer YAML (anchors, block scalars) is ignored rather than guessed
/// at, and callers skip unknown keys mermaid-style so a file carrying
/// config for other diagrams still renders.
fn frontmatter_section<'a>(source: &'a str, section: &str) -> Vec<(usize, &'a str)> {
    let mut out = Vec::new();
    let Some(body) = frontmatter_body(source) else {
        return out;
    };
    let indent_of = |l: &str| l.len() - l.trim_start().len();
    // Indent of `config:` / `<section>:` once seen. A line at or left
    // of a recorded indent has closed that block.
    let mut config_at: Option<usize> = None;
    // Indent shared by `config:`'s DIRECT children, learnt from the
    // first one seen. Without it a `sankey:` nested under some other
    // namespace (`config.themeVariables.sankey`) would reconfigure the
    // diagram.
    let mut child_at: Option<usize> = None;
    let mut section_at: Option<usize> = None;
    for line in body {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let ind = indent_of(line);
        if let Some(inner) = section_at {
            if ind > inner {
                out.push((ind, t));
                continue;
            }
            section_at = None;
        }
        if let Some(outer) = config_at {
            if ind > outer {
                let direct = *child_at.get_or_insert(ind);
                if ind == direct && t.strip_suffix(':') == Some(section) {
                    section_at = Some(ind);
                }
                continue;
            }
            config_at = None;
            child_at = None;
        }
        if t == "config:" {
            config_at = Some(ind);
        }
    }
    out
}

'''
s=s[:start]+new+s[end:]
open(p,'w').write(s)
