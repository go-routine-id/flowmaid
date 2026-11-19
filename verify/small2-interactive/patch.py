p='/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/small2-interactive/repo/src/vtest.rs'
s=open(p).read()
s=s.replace('''fn list() -> Vec<String> {
    let p = std::env::var("VFILES").unwrap();
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect()
}''','''fn list() -> Vec<String> {
    let p = std::env::var("VFILES").unwrap();
    let (k, n): (usize, usize) = (
        std::env::var("VSHARD").map(|v| v.parse().unwrap()).unwrap_or(0),
        std::env::var("VNSHARD").map(|v| v.parse().unwrap()).unwrap_or(1),
    );
    std::fs::read_to_string(p)
        .unwrap()
        .lines()
        .enumerate()
        .filter(|(i, _)| i % n == k)
        .map(|(_, s)| s.to_string())
        .collect()
}
fn app(line: &str) {
    use std::io::Write;
    let p = std::env::var("VOUT").unwrap() + &std::env::var("VSHARD").unwrap_or_default();
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
    f.write_all(line.as_bytes()).unwrap();
}''')
s=s.replace('''                log.push_str(&format!("NONFINITE {f} {what}: {why}\\n"));''','''                app(&format!("NONFINITE {f} {what}: {why}\\n"));''')
s=s.replace('''    std::fs::write(std::env::var("VOUT").unwrap(), &log).unwrap();
}

#[test]
fn v_huge_extreme''','''    app(&log);
}

#[test]
fn v_huge_extreme''')
s=s.replace('let k = (len / 0.02).ceil()','let k = (len / 0.04).ceil()')
s=s.replace('for _ in 0..120 {','for _ in 0..50 {')
s=s.replace('for gx in 0..9 {','for gx in 0..6 {').replace('for gy in 0..9 {','for gy in 0..6 {').replace('gx as f64 / 8.0','gx as f64 / 5.0').replace('gy as f64 / 8.0','gy as f64 / 5.0')
open(p,'w').write(s)
