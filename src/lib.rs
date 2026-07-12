use std::{collections::HashMap, ops::Neg};

pub use bls;
pub use brdb;

mod types;
#[macro_use]
mod misc;
mod events;
mod lights;
mod mappings;
mod prefabs;
mod prints;

use brdb::assets::materials::{GLOW, METALLIC, PLASTIC, TRANSLUCENT_PLASTIC};
use brdb::{
    Brick, BrickSize, BrickType, Collision, Direction, Entity, Guid, Owner, Position, Rotation,
    World,
};
use mappings::{BRICK_MAP_LITERAL, BRICK_MAP_REGEX};
use types::{BrickDesc, BrickMapping, Color};

const BRICK_OWNER: usize = 0;

pub struct ConvertReport {
    pub world: World,
    pub unknown_ui_names: HashMap<String, usize>,
    /// Blockland light datablocks we don't support yet, counted by `uiName`.
    pub unconverted_lights: HashMap<String, usize>,
    pub count_success: usize,
    pub count_failure: usize,
}

pub fn convert(save: &bls::Save) -> ConvertReport {
    let mut world = World::new();
    let mut owner_indices = HashMap::new();
    for bl_id in save.bricks.iter().filter_map(|brick| brick.owner) {
        owner_indices.entry(bl_id).or_insert_with(|| {
            let guid = blockland_owner_guid(bl_id);
            let name = format!("BL_ID {bl_id}");
            let (index, _) = world.owners.insert_full(
                guid,
                Owner {
                    user_id: guid,
                    user_name: name.clone(),
                    display_name: name,
                },
            );
            // brdb writes PUBLIC implicitly at on-disk index 0; World.owners
            // contains only the named owners that follow it.
            index + 1
        });
    }
    let sole_owner = if owner_indices.len() == 1 {
        owner_indices.values().next().copied()
    } else {
        None
    };
    world.meta.bundle.description = save.description.clone();

    // Resolve the Blockland palette to concrete RGBA colors once up front. brdb
    // has no per-save color palette, so each brick stores its color directly.
    let palette: Vec<Color> = save.colors.iter().map(|c| map_color(*c)).collect();

    let mut converter = Converter {
        world,
        unknown_ui_names: HashMap::new(),
        unconverted_lights: HashMap::new(),
    };

    let mut count_success = 0;
    let mut count_failure = 0;

    // Whether any text decal was emitted, so we register the font asset once.
    let mut used_font = false;

    let mut non_prio = Vec::new();
    // ModTer terrain must not share the main grid: Brickadia resolves overlaps
    // within a grid, which can make terrain disappear behind ordinary bricks.
    let mut modter_bricks = Vec::new();

    for from in &save.bricks {
        let owner_index = from
            .owner
            .and_then(|bl_id| owner_indices.get(&bl_id).copied())
            .or(sole_owner)
            .unwrap_or(BRICK_OWNER);

        // Bricks with a prefab-backed mapping bypass the BrickDesc pipeline:
        // the prefab template stamps its bricks (and any dynamic sub-grids,
        // wires, and joints) straight into the world.
        if let Some(template) = prefabs::template_for(&from.name) {
            let center = Position {
                x: (from.position.1 * 20.0) as i32,
                y: (from.position.0 * 20.0) as i32,
                z: (from.position.2 * 20.0) as i32,
            };
            let resolved = palette
                .get(from.color_index as usize)
                .copied()
                .unwrap_or(Color::from_rgba(255, 255, 255, 255));
            let (material, material_intensity) = resolve_material(resolved, from.color_fx);
            template.instantiate(
                &mut converter.world,
                center,
                from.angle,
                &prefabs::InstanceStyle {
                    color: resolved.into(),
                    material,
                    material_intensity,
                    collision: from.collision,
                    visible: from.rendering,
                    owner_index,
                },
            );
            count_success += 1;
            continue;
        }

        let option = converter.map_brick(from);

        let mappings = match option {
            Some(mappings) => {
                count_success += 1;
                mappings
            }
            None => {
                count_failure += 1;
                continue;
            }
        };

        // Resolve any lights hanging off this Blockland brick into point light
        // components. Unsupported datablocks are tallied for the report. They
        // attach to the first brick this mapping produces.
        let mut light_components: Vec<_> = from
            .lights()
            .iter()
            .filter_map(|light| match lights::map_light(&light.name) {
                Some(spec) => Some(lights::point_light_component(&spec)),
                None => {
                    *converter
                        .unconverted_lights
                        .entry(light.name.clone())
                        .or_default() += 1;
                    None
                }
            })
            .collect();

        // Stock letter prints (`Letters/A`) become a text decal on the brick.
        // Like lights, it rides on the first brick this mapping produces, and is
        // sized/faced from that brick's geometry.
        let ceiling = from.name.contains("Ceiling") || from.name.starts_with("Bottom Print");
        let print_size = mappings.first().map(|d| d.size);
        let mut text_component = from
            .print
            .as_deref()
            .filter(|_| prints::is_regular_print_brick(&from.name))
            .and_then(prints::letter_from_print)
            .map(|glyph| {
                let placement = print_size
                    .map(|size| prints::print_placement(size, ceiling))
                    .unwrap_or_default();
                prints::text_decal_component(glyph, placement, from.rendering)
            });

        // A `CenterPrint` event becomes a `Component_Interact` that shows its
        // message when the brick is clicked. Like lights/prints, it rides on the
        // first brick this mapping produces.
        let mut interact_component = from
            .events()
            .iter()
            .find_map(events::centerprint_message)
            .map(|msg| events::interact_component(&msg));

        for BrickDesc {
            asset,
            mut size,
            offset,
            rotation_offset,
            color_override,
            mut direction_override,
            non_priority,
            microwedge_rotate,
            inverted_modter_rotate,
            inverted_wedge_rotate,
            modter,
            rotate_by_direction,
            nocollide,
        } in mappings
        {
            let mut rotation = (from.angle + rotation_offset) % 4;

            let rotated_xy = rotate_offset((offset.0, offset.1), from.angle);
            let offset = (rotated_xy.0, rotated_xy.1, offset.2);

            let position = Position {
                x: (from.position.1 * 20.0) as i32 + offset.0,
                y: (from.position.0 * 20.0) as i32 + offset.1,
                z: (from.position.2 * 20.0) as i32 + offset.2,
            };

            // Resolve the final RGBA color: an explicit override wins, otherwise
            // look it up in the converted Blockland palette.
            let resolved = match color_override {
                Some(color) => color,
                None => palette
                    .get(from.color_index as usize)
                    .copied()
                    .unwrap_or(Color::from_rgba(255, 255, 255, 255)),
            };

            let (material, material_intensity) = resolve_material(resolved, from.color_fx);

            // convert a vertical slope to microwedge
            if microwedge_rotate {
                let original_dir = direction_override;
                let (x, y, z) = size;
                if rotation == 0 || rotation == 2 {
                    direction_override = Some(Direction::YPositive);
                    if rotation == 0 {
                        size = (z, x, y);
                    } else {
                        size = (x, z, y);
                        rotation = (rotation + 1) % 4;
                    }
                } else {
                    direction_override = Some(Direction::XPositive);
                    if rotation == 1 {
                        size = (x, z, y);
                        rotation = (rotation + 2) % 4;
                    } else {
                        size = (z, x, y);
                        rotation = (rotation + 1) % 4;
                    }
                }
                if matches!(original_dir, Some(Direction::ZNegative)) {
                    rotation = (rotation + 2) % 4;
                }
            }

            if rotate_by_direction {
                if rotation == 0 || rotation == 2 {
                    direction_override = if rotation == 0 {
                        Some(Direction::YPositive)
                    } else {
                        Some(Direction::YNegative)
                    };
                    let (x, y, z) = size;
                    size = (y, x, z);
                } else {
                    direction_override = if rotation == 1 {
                        Some(Direction::XNegative)
                    } else {
                        Some(Direction::XPositive)
                    };
                }
            }

            // fix odd rotation offsets on inverted ModTer, wedges
            if (inverted_modter_rotate && (rotation == 1 || rotation == 3))
                || (inverted_wedge_rotate && (rotation == 0 || rotation == 2))
            {
                rotation = (rotation + 2) % 4;
            }

            let collision_on = if from.collision { !nocollide } else { false };
            let collision = Collision {
                player: collision_on,
                weapon: collision_on,
                interact: collision_on,
                tool: collision_on,
                physics: collision_on,
                ..Default::default()
            };

            // Procedural (resizable) assets carry their size; static named bricks
            // (`B_...`) have a fixed size baked into the asset.
            let asset_type = if asset.starts_with("PB_") {
                BrickType::Procedural {
                    asset: asset.into(),
                    size: BrickSize::new(size.0 as u16, size.1 as u16, size.2 as u16),
                }
            } else {
                BrickType::Basic(asset.into())
            };

            let rotation = match rotation {
                0 => Rotation::Deg0,
                1 => Rotation::Deg90,
                2 => Rotation::Deg180,
                _ => Rotation::Deg270,
            };

            let mut brick = Brick {
                id: None,
                asset: asset_type,
                owner_index: Some(owner_index),
                original_owner_index: None,
                position,
                rotation,
                direction: direction_override.unwrap_or(Direction::ZPositive),
                collision,
                visible: from.rendering,
                color: resolved.into(),
                material,
                material_intensity,
                components: Vec::new(),
            };

            // Only the first brick a mapping produces carries the lights and
            // any text decal.
            for component in light_components.drain(..) {
                brick.add_component(component);
            }
            if let Some(text) = text_component.take() {
                brick.add_component(text);
                used_font = true;
            }
            if let Some(interact) = interact_component.take() {
                brick.add_component(interact);
            }

            if modter {
                modter_bricks.push(brick);
            } else if non_priority {
                non_prio.push(brick);
            } else {
                converter.world.bricks.push(brick);
            }
        }
    }

    converter.world.bricks.append(&mut non_prio);

    // An entity-backed grid has its own overlap domain. Freezing the grid keeps
    // the converted terrain stationary while retaining its original placement.
    if !modter_bricks.is_empty() {
        converter.world.add_brick_grid(
            Entity {
                frozen: true,
                ..Default::default()
            },
            modter_bricks,
        );
    }

    // Text decals reference the RobotoMono font by index into the world's
    // external asset table; register it (as index 0) before the writer resolves
    // the `Font` property. Only added when a decal actually uses it.
    if used_font {
        converter
            .world
            .global_data
            .external_asset_types
            .insert(prints::FONT_ASSET_TYPE.to_string());
        converter
            .world
            .global_data
            .external_asset_references
            .insert((
                prints::FONT_ASSET_TYPE.to_string(),
                prints::FONT_ASSET_NAME.to_string(),
            ));
    }

    // The writer requires every component type used on a brick to be registered
    // in the world's component schema and global-data tables. Do this after all
    // bricks (and their components) are in place.
    converter.world.register_used_components();

    ConvertReport {
        world: converter.world,
        unknown_ui_names: converter.unknown_ui_names,
        unconverted_lights: converter.unconverted_lights,
        count_success,
        count_failure,
    }
}

/// Put Blockland IDs in their own GUID namespace so they are stable between
/// conversions and cannot collide with Brickadia's all-zero PUBLIC owner.
fn blockland_owner_guid(bl_id: u64) -> Guid {
    Guid {
        a: 0x424c_534f, // "BLSO"
        b: 0,
        c: (bl_id >> 32) as u32,
        d: bl_id as u32,
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    use brdb::{Brz, IntoReader, OwnerTableSoA};

    #[test]
    fn preserves_blockland_builder_as_owner() {
        let mut save = bls::Save::new();
        let mut source = bls::Brick::new("1x1");
        source.owner = Some(2227);
        save.bricks.push(source);

        let report = convert(&save);
        assert_eq!(report.world.owners.len(), 1);
        let (guid, owner) = report.world.owners.get_index(0).unwrap();
        assert_eq!(*guid, blockland_owner_guid(2227));
        assert_eq!(owner.display_name, "BL_ID 2227");
        assert!(!report.world.bricks.is_empty());
        assert!(report.world.bricks.iter().all(|brick| {
            brick.owner_index == Some(1) && brick.original_owner_index.is_none()
        }));

        let bytes = report.world.to_brz_vec().unwrap();
        let reader = Brz::read_slice(&bytes).unwrap().into_reader();
        let owners = OwnerTableSoA::try_from(reader.owners_soa().unwrap()).unwrap();
        assert_eq!(owners.display_names, ["PUBLIC", "BL_ID 2227"]);
        let chunks = reader.brick_chunk_index(1).unwrap();
        for chunk in chunks {
            let soa = reader.brick_chunk_soa(1, chunk.index).unwrap();
            assert!(soa.owner_indices.iter().all(|&owner| owner == 1));
        }
    }

    #[test]
    fn sole_blockland_owner_applies_to_unstamped_bricks() {
        let mut save = bls::Save::new();
        let mut owned = bls::Brick::new("1x1");
        owned.owner = Some(2227);
        save.bricks.push(owned);
        save.bricks.push(bls::Brick::new("1x1"));

        let report = convert(&save);
        assert_eq!(report.world.bricks.len(), 2);
        assert!(report
            .world
            .bricks
            .iter()
            .all(|brick| brick.owner_index == Some(1)));
    }

    #[test]
    fn multiple_owners_do_not_guess_unstamped_brick_owner() {
        let mut save = bls::Save::new();
        for owner in [Some(2227), Some(999999), None] {
            let mut brick = bls::Brick::new("1x1");
            brick.owner = owner;
            save.bricks.push(brick);
        }

        let report = convert(&save);
        let owners: Vec<_> = report
            .world
            .bricks
            .iter()
            .map(|brick| brick.owner_index)
            .collect();
        assert_eq!(owners, [Some(1), Some(2), Some(BRICK_OWNER)]);
    }
}

struct Converter {
    world: World,
    unknown_ui_names: HashMap<String, usize>,
    unconverted_lights: HashMap<String, usize>,
}

impl Converter {
    fn map_brick(&mut self, from: &bls::Brick) -> Option<BrickMapping> {
        let mapping = map_brick(from);

        if cfg!(debug_assertions) {
            println!("mapped '{}' to {:?}", from.name, mapping);
        }

        if mapping.is_none() {
            *self.unknown_ui_names.entry(from.name.clone()).or_default() += 1;
        }

        mapping
    }
}

fn map_brick(from: &bls::Brick) -> Option<BrickMapping> {
    let ui_name = from.name.as_str();

    if let Some(mapping) = BRICK_MAP_LITERAL.get(ui_name) {
        return Some(mapping.clone());
    }

    for (regex, func) in BRICK_MAP_REGEX.iter() {
        if let Some(captures) = regex.captures(ui_name) {
            return func(captures, from);
        }
    }

    None
}

fn map_color(c: bls::Color) -> Color {
    let r = (c.r * 255.0).max(0.0).min(255.0) as u8;
    let g = (c.g * 255.0).max(0.0).min(255.0) as u8;
    let b = (c.b * 255.0).max(0.0).min(255.0) as u8;
    let a = (c.a * 255.0).max(0.0).min(255.0) as u8;

    Color::from_rgba(r, g, b, a)
}

/// Resolve a converted RGBA color and Blockland color effect into a Brickadia
/// material. Alpha carries meaning: opaque bricks use the material implied by
/// the color effect (plastic/glow/metallic); anything translucent becomes
/// translucent plastic. `material_intensity` is on a 0..=10 scale in the new
/// format (NOT the 0..=255 alpha range), so the alpha is rescaled; opaque
/// bricks get a neutral default intensity that glow brightness rides on.
fn resolve_material(resolved: Color, color_fx: u8) -> (brdb::BString, u8) {
    if resolved.a < 255 {
        (TRANSLUCENT_PLASTIC, alpha_to_intensity(resolved.a))
    } else {
        let material = match color_fx {
            3 => GLOW,
            1 | 2 => METALLIC,
            _ => PLASTIC,
        };
        (material, 5)
    }
}

/// Rescale a 0..=255 alpha byte to the new format's 0..=10 material intensity.
fn alpha_to_intensity(a: u8) -> u8 {
    ((a as u16 * 10 + 127) / 255).min(10) as u8
}

fn rotate_offset(mut offset: (i32, i32), angle: u8) -> (i32, i32) {
    for _ in 0..angle {
        offset = rotate_90_2d(offset);
    }
    offset
}

fn rotate_90_2d<X, Y: Neg>((x, y): (X, Y)) -> (<Y as Neg>::Output, X) {
    (-y, x)
}
