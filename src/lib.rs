use std::{collections::HashMap, ops::Neg};

pub use bls;
pub use brdb;

mod types;
#[macro_use]
mod misc;
mod mappings;

use brdb::assets::materials::{GLOW, METALLIC, PLASTIC, TRANSLUCENT_PLASTIC};
use brdb::{Brick, BrickSize, BrickType, Collision, Direction, Guid, Owner, Position, Rotation, World};
use mappings::{BRICK_MAP_LITERAL, BRICK_MAP_REGEX};
use types::{BrickDesc, BrickMapping, Color};

const BRICK_OWNER: usize = 0;

pub struct ConvertReport {
    pub world: World,
    pub unknown_ui_names: HashMap<String, usize>,
    pub count_success: usize,
    pub count_failure: usize,
}

pub fn convert(save: &bls::Save) -> ConvertReport {
    let mut world = World::new();
    world.owners.insert(Guid::default(), Owner::default());
    world.meta.bundle.description = save.description.clone();

    // Resolve the Blockland palette to concrete RGBA colors once up front. brdb
    // has no per-save color palette, so each brick stores its color directly.
    let palette: Vec<Color> = save.colors.iter().map(|c| map_color(*c)).collect();

    let mut converter = Converter {
        world,
        unknown_ui_names: HashMap::new(),
    };

    let mut count_success = 0;
    let mut count_failure = 0;

    let mut non_prio = Vec::new();

    for from in &save.bricks {
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

            // Alpha carries meaning: opaque bricks use the material implied by the
            // Blockland color effect; anything translucent becomes translucent
            // plastic. `material_intensity` is on a 0..=10 scale in the new format
            // (NOT the 0..=255 alpha range), so the alpha is rescaled.
            let (material, material_intensity) = if resolved.a < 255 {
                (TRANSLUCENT_PLASTIC, alpha_to_intensity(resolved.a))
            } else {
                let material = match from.color_fx {
                    3 => GLOW,
                    1 | 2 => METALLIC,
                    _ => PLASTIC,
                };
                // Neutral default intensity; glow brightness rides on this value.
                (material, 5)
            };

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

            let brick = Brick {
                id: None,
                asset: asset_type,
                owner_index: Some(BRICK_OWNER),
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

            if non_priority || (modter && !brick.visible) {
                non_prio.push(brick);
            } else {
                converter.world.bricks.push(brick);
            }
        }
    }

    converter.world.bricks.append(&mut non_prio);

    ConvertReport {
        world: converter.world,
        unknown_ui_names: converter.unknown_ui_names,
        count_success,
        count_failure,
    }
}

struct Converter {
    world: World,
    unknown_ui_names: HashMap<String, usize>,
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
    // Convert into Unreal color space
    let r = gamma_expansion(c.r);
    let g = gamma_expansion(c.g);
    let b = gamma_expansion(c.b);
    let a = gamma_expansion(c.a);

    // Convert to 0-255
    let r = (r * 255.0).max(0.0).min(255.0) as u8;
    let g = (g * 255.0).max(0.0).min(255.0) as u8;
    let b = (b * 255.0).max(0.0).min(255.0) as u8;
    let a = (a * 255.0).max(0.0).min(255.0) as u8;

    Color::from_rgba(r, g, b, a)
}

/// Rescale a 0..=255 alpha byte to the new format's 0..=10 material intensity.
fn alpha_to_intensity(a: u8) -> u8 {
    ((a as u16 * 10 + 127) / 255).min(10) as u8
}

fn gamma_expansion(u: f32) -> f32 {
    if u <= 0.04045 {
        return u / 12.92;
    }
    let base = (u + 0.055) / 1.055;
    base.powf(2.4)
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
