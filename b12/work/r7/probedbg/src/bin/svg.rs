//! svg <file.mmd> <out.svg>: the SVG (debug, scratch only)
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let src = std::fs::read_to_string(&a[1]).unwrap();
    std::fs::write(&a[2], flowmaid::render_svg(&src).unwrap()).unwrap();
}
