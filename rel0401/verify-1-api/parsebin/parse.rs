// flowmaid parse check: parse <cases.txt> (cases split by "\n%%%%\n") -> OK [names] | ERR line: msg
use flowmaid::model::Document;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let s = std::fs::read_to_string(&a[1]).unwrap();
    for c in s.split("\n%%%%\n") {
        let r = match flowmaid::parser::parse_document(c) {
            Ok(Document::Class(d)) => {
                let n: Vec<&str> = d.classes.iter().map(|c| c.name.as_str()).collect();
                format!("OK {n:?}")
            }
            Ok(_) => "OK other".to_string(),
            Err(e) => format!("ERR {}: {}", e.line, e.message),
        };
        println!("{:?}\t{}", c, r);
    }
}
