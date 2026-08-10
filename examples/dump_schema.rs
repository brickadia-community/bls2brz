//! Print a .brz's components schema (as parsed from the file itself).
//! Usage: dump_schema <file.brz>

use bls2brz::brdb::{Brz, IntoReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: dump_schema <file.brz>");
    let db = Brz::open(&path)?.into_reader();
    println!("{}", db.components_schema()?);
    Ok(())
}
