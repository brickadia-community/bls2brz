use brdb::{BrFsReader, Brz, IntoReader};

// Direct-reads a specific chunk. brdb 0.5's `brick_chunk_index` enumeration
// currently throws on multi-chunk saves, so pass the chunk index explicitly
// (see the file tree printed below for which chunks exist).
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: verify <file.brz> [cx cy cz]");
    let cx: i16 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let cy: i16 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);
    let cz: i16 = args.next().map(|s| s.parse().unwrap()).unwrap_or(0);

    let db = Brz::new(&path)?.into_reader();
    println!("{}", db.get_fs()?.render());

    let soa = db.brick_chunk_soa(1, (cx, cy, cz).into())?;
    println!(
        "chunk ({cx},{cy},{cz}): {} bricks, {} colors",
        soa.relative_positions.len(),
        soa.colors_and_alphas.len(),
    );
    let max_intensity = soa.colors_and_alphas.iter().map(|c| c.a).max().unwrap_or(0);
    let min_intensity = soa.colors_and_alphas.iter().map(|c| c.a).min().unwrap_or(0);
    println!("material_intensity (alpha byte) range: {min_intensity}..={max_intensity}");
    for (p, c) in soa.relative_positions.iter().zip(&soa.colors_and_alphas).take(3) {
        println!("  pos=({},{},{}) rgba({},{},{},{})", p.x, p.y, p.z, c.r, c.g, c.b, c.a);
    }
    Ok(())
}
