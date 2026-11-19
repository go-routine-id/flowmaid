use flowmaid::layout::{intrinsic_size_wrapped, text_width, wrap_label};
fn main() {
    let labels = ["Escalate the ticket to level two", "Notify the account manager", "Send the invoice to customer", "Archive the signed contract", "Check stock in the warehouse"];
    let src = format!("flowchart TD\n{}", labels.iter().enumerate().map(|(i, l)| format!("N{i}[{l}]")).collect::<Vec<_>>().join("\n"));
    let g = flowmaid::parser::parse(&src).unwrap();
    let real = |s: &str| text_width(s) * 1.15;
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(|n| intrinsic_size_wrapped(n, g.wrapping_width(), real)).collect();
    let sc = flowmaid::scene::scene_sized_with(&g, &sizes, real);
    for (i, n) in sc.nodes.iter().enumerate() {
        let own = wrap_label(&g.nodes[i], g.wrapping_width().unwrap(), real);
        let widest_real = n.label.lines().map(|l| real(l)).fold(0.0, f64::max);
        println!("{} (est {:.0}px): box {:.1}x{:.1} | SceneNode.label {:?} widest in real font {:.1}px vs box text room {:.1}px | embedder's own wrap {:?}", n.id, text_width(&g.nodes[i].label), n.w, n.h, n.label, widest_real, n.w - 2.0 * flowmaid::layout::PAD_X, own);
    }
}
