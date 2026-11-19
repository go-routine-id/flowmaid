// parsecase <cases.txt>: flowmaid parse of "classDiagram\n" + case (\n escaped)
use flowmaid::model::Document;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    for c in std::fs::read_to_string(&a[1]).unwrap().lines().filter(|l| !l.is_empty()) {
        let src = format!("classDiagram\n{}", c.replace("\\n", "\n"));
        let r = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Class(d)) => format!(
                "ok classes={:?}",
                d.classes.iter().map(|k| k.name.clone()).collect::<Vec<_>>()
            ),
            Ok(_) => "ok (other type)".into(),
            Err(e) => format!("ERR line {}: {}", e.line, e.message),
        };
        // Render too: must not panic.
        let rr = std::panic::catch_unwind(|| flowmaid::render_svg(&src).is_ok());
        println!("{c:?}\t{r}\trender_ok={:?}", rr);
    }
}
