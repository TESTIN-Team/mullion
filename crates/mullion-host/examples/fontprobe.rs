use mullion::text::FontBackend;
fn main() {
    let set = match mullion_host::GdiFontSet::new() {
        Some(s) => s,
        None => {
            println!("no dc");
            return;
        }
    };
    let m = set.metrics(15.0);
    println!("metrics: {:?}", m);
    for ch in ['A', '控', '中', ' ', '✓'] {
        match set.glyph(ch, 15.0) {
            Some(g) => println!(
                "{:?}: w={} h={} adv={} ink={}",
                ch,
                g.width,
                g.height,
                g.advance,
                g.alpha.iter().map(|&a| a as u32).sum::<u32>()
            ),
            None => println!("{:?}: NONE", ch),
        }
    }
}
