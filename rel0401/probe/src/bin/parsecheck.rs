// usage: parsecheck <cases.json> ; the JSON is a flat array of strings (as written by python json.dump)
use flowmaid::model::Document;
fn main() {
    let raw = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    // minimal JSON string-array reader
    let mut out = Vec::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut s = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => match chars.next().unwrap() {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'u' => {
                            let h: String = (0..4).map(|_| chars.next().unwrap()).collect();
                            s.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap()).unwrap());
                        }
                        o => s.push(o),
                    },
                    '"' => break,
                    o => s.push(o),
                }
            }
            out.push(s);
        }
    }
    for src in out {
        let r = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Class(d)) => format!(
                "OK {:?} notes={}",
                d.classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
                d.notes.len()
            ),
            Ok(_) => "OK (other)".to_string(),
            Err(e) => format!("ERR {}", e),
        };
        println!("{:?}\t{}", src, r);
    }
}
