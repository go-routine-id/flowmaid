//! Render one .mmd file to SVG on stdout: `cargo run -q --example svg -- file.mmd`
use std::io::Read;
fn main() {
    let path = std::env::args().nth(1).expect("usage: svg <file.mmd>");
    let mut src = String::new();
    std::fs::File::open(&path).unwrap().read_to_string(&mut src).unwrap();
    match flowmaid::render_svg(&src) {
        Ok(svg) => print!("{}", svg),
        Err(e) => { eprintln!("error: {:?}", e); std::process::exit(1); }
    }
}
