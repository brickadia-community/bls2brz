//! Read a `.brz` back through brdb, parsing every brick + component chunk the way
//! the game does, to surface schema/color errors locally.
//!
//! Usage: `cargo run --example read_check -- <file.brz>`

use bls2brz::brdb::{Brz, IntoReader};

fn main() {
    let path = std::env::args().nth(1).expect("usage: read_check <file.brz>");
    let reader = Brz::open(&path).expect("open brz").into_reader();

    use bls2brz::brdb::BrFsReader;
    eprintln!("{}", reader.get_fs().expect("fs").render());

    for grid_id in 0..64 {
        let chunks = match reader.brick_chunk_index(grid_id) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("grid {grid_id} chunk index error: {e}");
                continue;
            }
        };
        for meta in chunks {
            match reader.brick_chunk_soa(grid_id, meta.index) {
                Ok(soa) => {
                    eprintln!("grid {grid_id} chunk {}: {} bricks OK", meta.index, meta.num_bricks);
                    eprintln!("  material_indices: {:?}", soa.material_indices);
                    eprintln!("  colors_and_alphas: {:?}", &soa.colors_and_alphas);
                    eprintln!("  brick_type_indices: {:?}", soa.brick_type_indices);
                    eprintln!("  brick_sizes: {:?}", soa.brick_sizes);
                }
                Err(e) => eprintln!("BRICK ERROR grid {grid_id} chunk {}: {e}", meta.index),
            }
            if meta.num_components > 0 {
                match reader.component_chunk(grid_id, meta.index) {
                    Ok((_, data)) => {
                        eprintln!("  components OK: {} instances", data.len());
                        for (i, c) in data.iter().enumerate() {
                            let b = c.get("Brightness");
                            let r = c.get("Radius");
                            let color = c.get("Color").and_then(|v| match v {
                                bls2brz::brdb::schema::BrdbValue::Struct(s) => Some((
                                    s.get("R").cloned(), s.get("G").cloned(),
                                    s.get("B").cloned(), s.get("A").cloned(),
                                )),
                                _ => None,
                            });
                            eprintln!("  [{i}] B={b:?} R={r:?} color={color:?}");
                        }
                    }
                    Err(e) => eprintln!("  COMPONENT ERROR chunk {}: {e}", meta.index),
                }
            }
        }
    }
    eprintln!("done");
}
