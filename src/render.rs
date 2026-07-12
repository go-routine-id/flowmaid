//! Wrapper SVG: `render(graph)` = scene otomatis -> serialisasi SVG.
//! Seluruh logika geometri & serialisasi kini ada di modul `scene`,
//! sehingga output render() dan alur interaktif (scene/route/to_svg)
//! dijamin identik.

use crate::model::Graph;

pub fn render(g: &Graph) -> String {
    crate::scene::to_svg(&crate::scene::scene(g))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn paths(svg: &str) -> Vec<String> {
        svg.lines()
            .filter(|l| l.starts_with("<path d="))
            .map(|s| s.to_string())
            .collect()
    }

    fn attr(svg: &str, name: &str) -> f64 {
        let pat = format!("{}=\"", name);
        let i = svg.find(&pat).unwrap() + pat.len();
        let rest = &svg[i..];
        rest[..rest.find('"').unwrap()].parse().unwrap()
    }

    fn coords(path_line: &str) -> Vec<(f64, f64)> {
        let i = path_line.find("d=\"").unwrap() + 3;
        let rest = &path_line[i..];
        let d = &rest[..rest.find('"').unwrap()];
        let nums: Vec<f64> = d
            .split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap())
            .collect();
        nums.chunks(2).map(|c| (c[0], c[1])).collect()
    }

    #[test]
    fn edge_paralel_terpisah() {
        let g = parse("A -->|x| B\nA -->|y| B").unwrap();
        let svg = render(&g);
        let p = paths(&svg);
        assert_eq!(p.len(), 2);
        assert_ne!(p[0], p[1], "edge paralel tidak boleh menumpuk persis");
    }

    #[test]
    fn semua_titik_kurva_di_dalam_kanvas() {
        let g = parse("flowchart TD\nA --> B\nB --> B\nB --> C\nC --> A").unwrap();
        let svg = render(&g);
        let w = attr(&svg, "width");
        let h = attr(&svg, "height");
        for line in svg.lines().filter(|l| l.starts_with("<path d=")) {
            for (x, y) in coords(line) {
                assert!(x >= -0.5 && x <= w + 0.5, "x={} keluar kanvas w={}", x, w);
                assert!(y >= -0.5 && y <= h + 0.5, "y={} keluar kanvas h={}", y, h);
            }
        }
    }

    #[test]
    fn arah_bt_dan_rl_tidak_panik() {
        for d in ["BT", "RL"] {
            let src = format!(
                "flowchart {}\nA[Mulai] --> B{{Cek}}\nB -->|ya| C((Ok))\nC --> A",
                d
            );
            let svg = render(&parse(&src).unwrap());
            assert!(svg.contains("Mulai") && svg.contains("</svg>"));
        }
    }

    #[test]
    fn fanout_titik_keluar_menyebar() {
        let g = parse("A[Induk Lebar Sekali] --> B\nA --> C\nA --> D").unwrap();
        let svg = render(&g);
        let starts: Vec<(f64, f64)> = paths(&svg).iter().map(|p| coords(p)[0]).collect();
        assert_eq!(starts.len(), 3);
        assert!(
            starts[0] != starts[1] && starts[1] != starts[2] && starts[0] != starts[2],
            "titik keluar harus menyebar: {:?}",
            starts
        );
    }

    #[test]
    fn back_edge_mengitari_layer_terlebar() {
        let src = "flowchart TD\nA --> B1\nA --> B2\nA --> B3\nB1 --> C\nB2 --> C\nB3 --> C\nC --> A";
        let g = parse(src).unwrap();
        let svg = render(&g);
        let back = paths(&svg).pop().unwrap(); // edge terakhir = C --> A
        let c = coords(&back);
        let apex_x = 0.125 * (c[0].0 + c[3].0) + 0.75 * c[1].0;
        let mut min_left = f64::INFINITY;
        let mut max_right = f64::NEG_INFINITY;
        for line in svg.lines().filter(|l| l.starts_with("<rect x=")) {
            let x = attr(line, "x");
            let w = attr(line, "width");
            min_left = min_left.min(x);
            max_right = max_right.max(x + w);
        }
        assert!(
            apex_x > max_right + 8.0 || apex_x < min_left - 8.0,
            "puncak back-edge ({:.0}) harus di luar node [{:.0}..{:.0}]",
            apex_x,
            min_left,
            max_right
        );
    }
}
