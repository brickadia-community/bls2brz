//! Dump the raw bytes of a grid's component chunk .mps for byte-level comparison.
//! Usage: raw_component <file.brz> <grid> <chunk-name>   e.g. ... 1 -4_-1_0

use bls2brz::brdb::{BrFsReader, Brz, IntoReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut a = std::env::args().skip(1);
    let path = a.next().expect("file");
    let grid = a.next().expect("grid");
    let chunk = a.next().expect("chunk name e.g. -4_-1_0");
    let reader = Brz::open(&path)?.into_reader();
    let p = format!("World/0/Bricks/Grids/{grid}/Components/{chunk}.mps");
    let bytes = reader.read_file(&p)?;
    println!("{} bytes: {}", p, bytes.len());
    for (i, chunk) in bytes.chunks(16).enumerate() {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let asc: String = chunk
            .iter()
            .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '.' })
            .collect();
        println!("{:04x}  {:<48}  {}", i * 16, hex.join(" "), asc);
    }
    Ok(())
}
