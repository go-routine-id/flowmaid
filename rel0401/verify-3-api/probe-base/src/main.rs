#[path = "../../psrc/common.rs"]
mod common;
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let args: Vec<String> = std::env::args().collect();
    if !common::run_common(&args) {
        eprintln!("unknown command");
    }
}
