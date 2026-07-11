use bls2brz::brdb::{BrReader, Brz, IntoReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let p = std::env::args().nth(1).expect("usage: dump_components <file.brz>");
    let db: BrReader<_> = Brz::open(&p)?.into_reader();
    let gd = db.global_data()?;
    let types = &gd.component_type_names;
    println!("Component types: {:?}", types);
    for gid in 0..8 {
        let chunks = match db.brick_chunk_index(gid) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for chunk in chunks {
            if chunk.num_components == 0 {
                continue;
            }
            let (soa, _components) = db.component_chunk_soa(gid, chunk.index)?;
            // Resolve counter type indices to names, show run structure.
            let runs: Vec<(String, u32)> = soa
                .component_type_counters
                .iter()
                .map(|c| {
                    let name = types
                        .get_index(c.type_index as usize)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("#{}", c.type_index));
                    (name, c.num_instances)
                })
                .collect();
            println!(
                "grid {gid} chunk {}: {} comps, {} runs",
                chunk.index,
                soa.component_brick_indices.len(),
                runs.len()
            );
            println!("  runs: {:?}", runs);
            println!("  brick_indices: {:?}", soa.component_brick_indices);
        }
    }
    Ok(())
}
