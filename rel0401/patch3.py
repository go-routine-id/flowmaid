import sys
W='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0401/wt/'
def sub(path, old, new, count=1):
    p=W+path; s=open(p).read(); n=s.count(old)
    if n!=count: sys.exit(f"{path}: expected {count} got {n}: {old[:80]!r}")
    open(p,'w').write(s.replace(old,new))

sub('src/parser.rs','''            // `class Name {` opens a block; `class Name` just declares.
            if let Some(name) = rest.strip_suffix('{') {
                let ci = declare_class(&mut d, name.trim());
                open = Some((ci, lineno));''','''            // `class Name {` opens a block; `class Name` just declares.
            if let Some(name) = rest.strip_suffix('{') {
                check_declared_name(name, lineno)?;
                let ci = declare_class(&mut d, name.trim());
                open = Some((ci, lineno));''')
sub('src/parser.rs','''                for name in names.map(str::trim).filter(|n| !n.is_empty()) {
                    declare_class(&mut d, name);
                }''','''                for name in names.map(str::trim).filter(|n| !n.is_empty()) {
                    check_declared_name(name, lineno)?;
                    declare_class(&mut d, name);
                }''')
sub('src/parser.rs','''            let (a, name) = (a.0.trim(), a.1.trim());
            if !a.is_empty() && !name.is_empty() {
                let ci = declare_class(&mut d, name);''','''            let (a, name) = (a.0.trim(), a.1.trim());
            if !a.is_empty() && !name.is_empty() {
                check_declared_name(name, lineno)?;
                let ci = declare_class(&mut d, name);''')
sub('src/parser.rs','''/// The class a `class …` / `<<stereotype>> …` statement declares. Any
/// text is taken as a name there, as it always was; a [`class_name`]
/// spelling is read as one (backticks stripped, generic split off).''','''/// The class a `class …` / `<<stereotype>> …` statement declares. Any
/// text is taken as a name there, as it always was, but for a stray dot
/// ([`check_declared_name`]); a [`class_name`] spelling is read as one
/// (backticks stripped, generic split off).''')
sub('src/parser.rs','''/// The stereotype of a `<<interface>>` member line, without guillemets.''','''/// Mermaid's parse error for a `class …` / `<<stereotype>> …` statement
/// whose name starts or ends with a dot or has two in a row (`A.`, `.A`,
/// `A..B`, `A...B`, `A.B.`): a dot only joins the parts of a dotted name
/// ([`is_bare_class_name`], `a.b.C`) and `..` is the dotted line, so
/// mermaid rejects such a declaration where flowmaid used to take the
/// name as written. A backtick name (`` `A.` ``) is taken as written, as
/// in mermaid, and only the name itself is read, before its generic
/// (`~T~`), label (`["…"]`) or `:::` style.
fn check_declared_name(name: &str, lineno: usize) -> Result<(), ParseError> {
    let name = name.trim();
    if name.starts_with('`') {
        return Ok(());
    }
    let end = ["~", "[", ":::"]
        .iter()
        .filter_map(|m| name.find(m))
        .min()
        .unwrap_or(name.len());
    let bare = &name[..end];
    if bare.contains('.') && bare.split('.').any(|part| part.trim().is_empty()) {
        return Err(err(
            lineno,
            format!(
                "'{bare}' is no class name: a dot joins two parts of a name, \\
                 as in 'com.example.Order', and '..' is the dotted line"
            ),
        ));
    }
    Ok(())
}

/// The stereotype of a `<<interface>>` member line, without guillemets.''')

sub('src/class.rs','''    /// Backtick names are mermaid's literal class names: the text''','''    /// A `class` (or `<<stereotype>>`) statement whose name starts or ends
    /// with a dot or has two in a row is a parse error, as in mermaid 12.2
    /// (each form below checked against it); the dotted names mermaid
    /// takes, and backtick names, still parse.
    #[test]
    fn declared_names_with_a_stray_dot_are_errors() {
        for (bad, line) in [
            ("class A.", 2),
            ("class .A", 2),
            ("class A..B", 2),
            ("class A...B", 2),
            ("class A....B", 2),
            ("class A..", 2),
            ("class ..A", 2),
            ("class A.B.", 2),
            ("class .A.B", 2),
            ("class A..B.C", 2),
            ("class .", 2),
            ("class A .", 2),
            ("class  .A", 2),
            ("class A . . B", 2),
            ("class A-.", 2),
            ("class A.~T~", 2),
            ("class A.B.~T~", 2),
            ("class A..B~T~", 2),
            ("class A.:::foo", 2),
            ("class A.B.:::foo", 2),
            ("class A..B:::foo", 2),
            ("class A.[\\"lbl\\"]", 2),
            ("class A., B", 2),
            ("class A, B.", 2),
            ("class A, .B", 2),
            ("class A. {\\n}", 2),
            ("class .A {\\n}", 2),
            ("class A...B {\\n}", 2),
            ("<<interface>> A.", 2),
            ("<<interface>> .A", 2),
            ("class A\\nclass B.", 3),
            ("class A.\\nA --> B", 2),
        ] {
            let e = parse_document(&format!("classDiagram\\n{bad}"))
                .err()
                .unwrap_or_else(|| panic!("{bad:?} parsed"));
            assert_eq!(e.line, line, "{bad:?}: {e}");
            assert!(e.message.contains("is no class name"), "{bad:?}: {e}");
        }
        // The error names what it read as the name.
        let e = parse_document("classDiagram\\nclass A..B~T~").unwrap_err();
        assert!(e.message.starts_with("'A..B' is no class name"), "{e}");
        for (good, names) in [
            ("class A.B", vec!["A.B"]),
            ("class a.b.C", vec!["a.b.C"]),
            ("class A.-B", vec!["A.-B"]),
            ("class A-.B", vec!["A-.B"]),
            ("class A._B", vec!["A._B"]),
            ("class A.1", vec!["A.1"]),
            ("class 1.A", vec!["1.A"]),
            ("class A.-", vec!["A.-"]),
            ("class A.B-", vec!["A.B-"]),
            ("class A.B~T~", vec!["A.B"]),
            ("class Map~a.b~", vec!["Map"]),
            ("class A.B {\\n+x int\\n}", vec!["A.B"]),
            ("class A.B.C {\\n}", vec!["A.B.C"]),
            ("class `A.`", vec!["A."]),
            ("class `.A`", vec![".A"]),
            ("class `A..B`", vec!["A..B"]),
            ("class A.B {\\n}\\n<<interface>> A.B", vec!["A.B"]),
            ("A.B --> C", vec!["A.B", "C"]),
            ("A..B", vec!["A", "B"]),
        ] {
            let d = cd(&format!("classDiagram\\n{good}"));
            let got: Vec<&str> = d.classes.iter().map(|c| c.name.as_str()).collect();
            assert_eq!(got, names, "{good:?}");
        }
        // Mermaid takes these too (spaces round a dot, a label, a style);
        // flowmaid keeps parsing them as it did.
        for good in [
            "class A. B",
            "class A .B",
            "class A . B",
            "class A.B[\\"lbl\\"]",
            "class A.B[\\"x.y\\"]",
            "class A[\\"x.\\"]",
            "class A.B:::foo",
        ] {
            assert!(
                parse_document(&format!("classDiagram\\n{good}")).is_ok(),
                "{good:?}"
            );
        }
    }

    /// Backtick names are mermaid's literal class names: the text''')

sub('CHANGELOG.md','''  `viewBox` stays at `0 0`; the others draw the same picture as before,
  with the offset in the `viewBox` instead of in every coordinate.
''','''  `viewBox` stays at `0 0`; the others draw the same picture as before,
  with the offset in the `viewBox` instead of in every coordinate.
- **Class diagrams: a declared name with a stray dot is an error, as in
  mermaid.** `class A.`, `class .A`, `class A..B`, `class A...B`, `class
  A.B.` (and the same in a `class … {` block, a `class A, B.` list, a
  name with a generic, label or `:::` style, and `<<interface>> A.`)
  give a parse error naming the line, as mermaid 12.2 does; they were
  taken as a class of that name. Dotted names mermaid takes still work
  (`class a.b.C`, `class A.B~T~`, `class A.-B`), and so do backtick
  names (`` class `A.` ``).
''')
print('ok')
