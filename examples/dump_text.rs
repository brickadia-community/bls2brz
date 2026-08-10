//! Dump every field of every component struct in a .brz.
//! Usage: dump_text <file.brz>

use bls2brz::brdb::schema::{BrdbSchema, BrdbValue};
use bls2brz::brdb::{Brz, IntoReader};

fn shallow(value: &BrdbValue, schema: &BrdbSchema) -> String {
    match value {
        BrdbValue::Struct(s) => {
            let name = s.get_name();
            let inner: Vec<String> = schema
                .get_struct(name)
                .map(|def| {
                    def.keys()
                        .filter_map(|p| p.get(schema))
                        .map(|p| format!("{p}={:?}", s.get(p).map(|v| format!("{v:?}"))))
                        .collect()
                })
                .unwrap_or_default();
            format!("{name} {{ {} }}", inner.join(", "))
        }
        BrdbValue::Enum(e) => format!(
            "{}::{:?}",
            e.name.get(schema).unwrap_or_default(),
            e.value
        ),
        v => format!("{v:?}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: dump_text <file.brz>");
    let db = Brz::open(&path)?.into_reader();
    let data = db.global_data()?;
    let schema = db.components_schema()?;

    let _ = &schema;

    let mut grid_ids = vec![1usize];
    for index in db.entity_chunk_index()? {
        for e in db.entity_chunk(index)? {
            if e.is_brick_grid() {
                if let Some(id) = e.id {
                    grid_ids.push(id);
                }
            }
        }
    }

    for gid in grid_ids {
        for chunk in db.brick_chunk_index(gid)? {
            if chunk.num_components == 0 {
                continue;
            }
            let (csoa, structs) = db.component_chunk_soa(gid, chunk.index)?;
            println!("grid {gid} chunk {} bricks {:?}", chunk.index, csoa.component_brick_indices);
            let _ = &data;
            for s in &structs {
                println!("  {}", s.get_name());
                let Some(def) = schema.get_struct(s.get_name()) else {
                    continue;
                };
                for prop_id in def.keys() {
                    let Some(prop) = prop_id.get(&schema) else { continue };
                    let Some(value) = s.get(prop) else {
                        println!("    {prop} = <missing>");
                        continue;
                    };
                    println!("    {prop} = {}", shallow(value, &schema));
                }
            }
        }
    }
    Ok(())
}
