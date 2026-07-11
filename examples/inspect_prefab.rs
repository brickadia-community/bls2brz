//! Dump the full structure of a prefab .brz: entities, grids, bricks,
//! components (with data), and wires.
//! Usage: inspect_prefab <file.brz>

use bls2brz::brdb::{Brz, IntoReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: inspect_prefab <file.brz>");
    let db = Brz::open(&path)?.into_reader();
    let data = db.global_data()?;

    println!("=== Global data ===");
    println!("Basic brick assets: {:?}", data.basic_brick_asset_names);
    println!("Procedural assets: {:?}", data.procedural_brick_asset_names);
    println!("Component types: {:?}", data.component_type_names);
    println!(
        "Component structs: {:?}",
        data.component_data_struct_names
    );
    println!("Wire port names: {:?}", data.component_wire_port_names);
    println!("Entity types: {:?}", data.entity_type_names);

    println!("\n=== Schemas ===");
    println!("Entities schema:\n{}", db.entities_schema()?);

    println!("\n=== Entities ===");
    let mut grid_ids = vec![1usize];
    for index in db.entity_chunk_index()? {
        for e in db.entity_chunk(index)? {
            println!(
                "entity id={:?} asset={} loc={:?} rot={:?} frozen={} owner={:?}",
                e.id, e.asset, e.location, e.rotation, e.frozen, e.owner_index
            );
            if e.is_brick_grid() {
                if let Some(id) = e.id {
                    grid_ids.push(id);
                }
            }
        }
    }

    for gid in grid_ids {
        println!("\n=== Grid {gid} ===");
        let chunks = match db.brick_chunk_index(gid) {
            Ok(c) => c,
            Err(e) => {
                println!("  (no chunk index: {e})");
                continue;
            }
        };
        for chunk in chunks {
            println!(
                "chunk {} bricks={} components={} wires={}",
                chunk.index, chunk.num_bricks, chunk.num_components, chunk.num_wires
            );
            let soa = db.brick_chunk_soa(gid, chunk.index)?;
            for (i, brick) in soa.iter_bricks(chunk.index, data.clone()).enumerate() {
                let b = brick?;
                println!(
                    "  brick[{i}] asset={:?} pos={:?} rot={:?} dir={:?} color={:?} mat={:?}/{} vis={} col={:?}",
                    b.asset,
                    b.position,
                    b.rotation,
                    b.direction,
                    b.color,
                    b.material,
                    b.material_intensity,
                    b.visible,
                    b.collision,
                );
            }
            if chunk.num_components > 0 {
                let (csoa, structs) = db.component_chunk_soa(gid, chunk.index)?;
                println!("  component brick indices: {:?}", csoa.component_brick_indices);
                let runs: Vec<(String, u32)> = csoa
                    .component_type_counters
                    .iter()
                    .map(|c| {
                        let name = data
                            .component_type_names
                            .get_index(c.type_index as usize)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("#{}", c.type_index));
                        (name, c.num_instances)
                    })
                    .collect();
                println!("  component runs: {runs:?}");
                println!("  joint brick indices: {:?}", csoa.joint_brick_indices);
                println!("  joint entity refs: {:?}", csoa.joint_entity_references);
                println!(
                    "  joint rel offsets: {:?}",
                    csoa.joint_initial_relative_offsets
                );
                println!(
                    "  joint rel rotations: {:?}",
                    csoa.joint_initial_relative_rotations
                );
                println!(
                    "  microchip brick indices: {:?}",
                    csoa.microchip_brick_indices
                );
                const FIELDS: &[&str] = &[
                    "InteractSound",
                    "bAllowNearbyInteraction",
                    "bHiddenInteraction",
                    "PromptCustomLabel",
                    "bEnabled",
                    "Blend",
                    "bClampAlpha",
                    "InputA",
                    "InputB",
                    "bInputA",
                    "bInputB",
                    "TargetAngle",
                    "Power",
                    "ActiveDamping",
                    "ForceLimit",
                    "bLimitAngle",
                    "LimitAngle",
                    "CurrentAngle",
                    "bAnglesArePercentages",
                    "bReversed",
                    "Damping",
                ];
                for s in &structs {
                    println!("  component data: {}", s.get_name());
                    for f in FIELDS {
                        if let Some(v) = s.get(f) {
                            println!("    {f} = {v:?}");
                        }
                    }
                }
            }
            if chunk.num_wires > 0 {
                let wires = db.wire_chunk_soa(gid, chunk.index)?;
                println!("  wires: {wires}");
            }
        }
    }

    Ok(())
}
