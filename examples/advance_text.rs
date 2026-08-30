//! Render one advance text-DSL file to SVG on stdout:
//! `cargo run -q --example advance_text -- advance_swimlane.mmd`
use std::io::Read;
fn main() {
    let path = std::env::args().nth(1).expect("usage: advance_text <file.mmd>");
    let mut src = String::new();
    std::fs::File::open(&path).unwrap().read_to_string(&mut src).unwrap();
    match flowmaid::render_advance_text_svg(&src) {
        Ok(svg) => print!("{}", svg),
        Err(e) => {
            eprintln!("advance text error: {}", e);
            std::process::exit(1);
        }
    }
}
