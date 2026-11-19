fn main() {
    for s in std::env::args().skip(1) { println!("{:8.2} {}", flowmaid::layout::text_width(&s), s); }
}
