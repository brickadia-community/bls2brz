//! Read component chunks exactly like the game: per counter run, read
//! `num_instances` structs of that type sequentially. Reports the first desync.

use bls2brz::brdb::schema::{as_brdb::AsBrdbValue, ReadBrdbSchema};
use bls2brz::brdb::{BrFsReader, Brz, IntoReader};

const BRICK_COMPONENT_SOA: &str = "BRSavedComponentChunkSoA";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: strict_read <file.brz>");
    let reader = Brz::open(&path)?.into_reader();
    let gd = reader.global_data()?;
    let struct_names = &gd.component_data_struct_names;

    let schema_bytes = reader.read_file("World/0/Bricks/ComponentsShared.schema")?;
    let comp_schema = (&mut schema_bytes.as_slice()).read_brdb_schema_with_data(gd.clone())?;

    for gid in 0..64 {
        let metas = match reader.brick_chunk_index(gid) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for meta in metas {
            if meta.num_components == 0 {
                continue;
            }
            let p = format!("World/0/Bricks/Grids/{gid}/Components/{}.mps", meta.index);
            let bytes = reader.read_file(&p)?;
            let buf = &mut bytes.as_slice();
            let chunk = buf.read_brdb(&comp_schema, BRICK_COMPONENT_SOA)?;
            let counters = chunk.prop("ComponentTypeCounters")?.as_array()?;
            let mut comp_i = 0usize;
            for counter in counters {
                let s = counter.as_struct()?;
                let type_idx = s.prop("TypeIndex")?.as_brdb_u32()? as usize;
                let num = s.prop("NumInstances")?.as_brdb_u32()? as usize;
                let sname = struct_names.get(type_idx).map(|s| s.as_str()).unwrap_or("");
                if sname.is_empty() || sname == "None" {
                    comp_i += num;
                    continue;
                }
                for k in 0..num {
                    if let Err(e) = buf.read_brdb(&comp_schema, sname) {
                        println!(
                            "DESYNC grid {gid} chunk {} type {sname} instance {k}/{num} (component #{comp_i}): {e}",
                            meta.index
                        );
                        return Ok(());
                    }
                    comp_i += 1;
                }
            }
        }
    }
    println!("all components read strictly OK");
    Ok(())
}
