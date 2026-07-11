//! Prefab-backed brick mappings.
//!
//! Instead of hand-coding a multi-brick mapping in `mappings.rs`, a Blockland
//! brick can be mapped to a Brickadia prefab (`.brz`) built in-game and
//! embedded in this binary. The prefab is parsed once into a [`PrefabTemplate`]
//! — bricks relative to the prefab's center, plus components, wires, joints,
//! and dynamic sub-grids — and then stamped into the output world once per
//! Blockland brick, rotated to match the brick's angle.
//!
//! The prefab is baked into the world rather than placed as a prefab asset:
//! main-grid bricks land on the main grid, and any dynamic grid in the prefab
//! (e.g. a servo-driven door panel) becomes a fresh dynamic grid entity jointed
//! to the instance's joint brick.

use std::collections::HashMap;

use brdb::{
    assets::LiteralComponent, AsBrdbValue, BString, Brick, BrickType, Brz, Collision, Direction,
    Entity, EntityColors, IntoReader, LocalWirePortSource, Position, Quat4f, RemoteWirePortSource,
    Rotation, Vector3f, WirePort, WirePortTarget, World,
};
use brdb::schema::BrdbValue;
use lazy_static::lazy_static;

/// The `Plain Door.brz` prefab: a door frame on the main grid whose panel is a
/// servo-driven dynamic grid, opened by clicking switch components on the
/// panel and handles.
pub static PLAIN_DOOR_BRZ: &[u8] = include_bytes!("../prefabs/plain_door.brz");

lazy_static! {
    static ref PLAIN_DOOR: PrefabTemplate = {
        let mut t = PrefabTemplate::parse(PLAIN_DOOR_BRZ).expect("parse embedded plain_door.brz");
        t.angle_offset = 0;
        t
    };
}

/// Return the prefab template mapped to a Blockland `uiName`, if any.
pub fn template_for(ui_name: &str) -> Option<&'static PrefabTemplate> {
    match ui_name {
        "Plain Door" | "House Door" => Some(&PLAIN_DOOR),
        _ => None,
    }
}

/// A field value captured from a prefab component. Asset references are stored
/// by name so they can be re-registered (and re-indexed) in the target world.
enum TemplateValue {
    Value(BrdbValue),
    /// External asset reference: `(asset_type, asset_name)`; `None` is a null
    /// reference.
    Asset(Option<(String, String)>),
}

struct TemplateComponent {
    type_name: String,
    fields: Vec<(String, TemplateValue)>,
}

struct TemplateBrick {
    asset: BrickType,
    /// Main-grid bricks: relative to the prefab anchor. Sub-grid bricks:
    /// grid-local coordinates.
    position: Position,
    rotation: Rotation,
    direction: Direction,
    visible: bool,
    collision: Collision,
    components: Vec<TemplateComponent>,
}

struct TemplateGrid {
    bricks: Vec<TemplateBrick>,
    colors: EntityColors,
}

/// A brick either on the prefab's main grid or in one of its sub-grids.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum BrickRef {
    Main(usize),
    Grid { grid: usize, brick: usize },
}

struct TemplateWireEnd {
    brick: BrickRef,
    component_type: String,
    port: String,
}

struct TemplateWire {
    source: TemplateWireEnd,
    target: TemplateWireEnd,
}

struct TemplateJoint {
    main_brick: usize,
    grid: usize,
    relative_offset: Vector3f,
    relative_rotation: Quat4f,
}

pub struct PrefabTemplate {
    main_bricks: Vec<TemplateBrick>,
    grids: Vec<TemplateGrid>,
    wires: Vec<TemplateWire>,
    joints: Vec<TemplateJoint>,
    /// Number of extra 90° steps between a Blockland brick at angle 0 and the
    /// orientation the prefab was built at. Calibrated per prefab so instances
    /// face the same way the hand-coded mapping did.
    angle_offset: u8,
}

impl PrefabTemplate {
    fn parse(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Brz::read_slice(bytes)?.into_reader();
        let data = db.global_data()?;
        let components_schema = db.components_schema()?;

        let external_assets: Vec<(String, String)> = data
            .external_asset_references
            .iter()
            .cloned()
            .collect();

        // Sub-grid entity persistent index -> template grid slot.
        let mut grid_slots: HashMap<usize, usize> = HashMap::new();
        let mut grids: Vec<TemplateGrid> = Vec::new();
        for index in db.entity_chunk_index()? {
            for e in db.entity_chunk(index)? {
                if !e.is_brick_grid() {
                    continue;
                }
                let Some(id) = e.id else { continue };
                grid_slots.insert(id, grids.len());
                grids.push(TemplateGrid {
                    bricks: Vec::new(),
                    colors: e.color_and_alpha.clone(),
                });
            }
        }

        let mut main_bricks: Vec<TemplateBrick> = Vec::new();
        let mut raw_wires: Vec<RawWire> = Vec::new();
        let mut joints: Vec<(usize, u32, Vector3f, Quat4f)> = Vec::new();

        // Prefab (grid id, chunk, brick index in chunk) -> BrickRef, needed to
        // resolve wire endpoints after all bricks are collected.
        let mut brick_refs: HashMap<(usize, (i16, i16, i16), u32), BrickRef> = HashMap::new();

        let component_type_name = |idx: u16| -> Result<String, String> {
            data.component_type_names
                .get_index(idx as usize)
                .cloned()
                .ok_or_else(|| format!("unknown component type index {idx}"))
        };
        let port_name = |idx: u16| -> Result<String, String> {
            data.component_wire_port_names
                .get_index(idx as usize)
                .cloned()
                .ok_or_else(|| format!("unknown wire port index {idx}"))
        };

        let mut grid_ids: Vec<usize> = vec![1];
        grid_ids.extend(grid_slots.keys().copied());

        for gid in grid_ids {
            for chunk in db.brick_chunk_index(gid)? {
                let chunk_key = (chunk.index.x, chunk.index.y, chunk.index.z);
                let soa = db.brick_chunk_soa(gid, chunk.index)?;

                let mut chunk_bricks: Vec<TemplateBrick> = Vec::new();
                for brick in soa.iter_bricks(chunk.index, data.clone()) {
                    let b = brick?;
                    let position = if gid == 1 {
                        b.position
                    } else {
                        // World::add_brick_grid re-centers sub-grid bricks by
                        // subtracting CHUNK_HALF; undo the stored shift so the
                        // template holds true grid-local coordinates.
                        b.position + Position::CHUNK_HALF
                    };
                    chunk_bricks.push(TemplateBrick {
                        asset: b.asset,
                        position,
                        rotation: b.rotation,
                        direction: b.direction,
                        visible: b.visible,
                        collision: b.collision,
                        components: Vec::new(),
                    });
                }

                if chunk.num_components > 0 {
                    let (csoa, structs) = db.component_chunk_soa(gid, chunk.index)?;

                    // Component instances are stored grouped by type;
                    // component_brick_indices runs parallel to the expanded
                    // counters, and the struct list only contains entries for
                    // types with a data struct.
                    let mut brick_idx_iter = csoa.component_brick_indices.iter();
                    let mut struct_iter = structs.iter();
                    for counter in &csoa.component_type_counters {
                        let type_name = component_type_name(counter.type_index as u16)?;
                        let has_struct = data
                            .component_data_struct_names
                            .get(counter.type_index as usize)
                            .map(|s| s != "None")
                            .unwrap_or(false);
                        for _ in 0..counter.num_instances {
                            let &brick_index = brick_idx_iter
                                .next()
                                .ok_or("component brick indices exhausted")?;
                            let mut fields = Vec::new();
                            if has_struct {
                                let s = struct_iter.next().ok_or("component structs exhausted")?;
                                let struct_def = components_schema
                                    .get_struct(s.get_name())
                                    .ok_or_else(|| {
                                        format!("unknown component struct {}", s.get_name())
                                    })?;
                                for prop_id in struct_def.keys() {
                                    let Some(prop) = prop_id.get(&components_schema) else {
                                        continue;
                                    };
                                    let Some(value) = s.get(prop) else { continue };
                                    let value = match value {
                                        BrdbValue::Asset(idx) => TemplateValue::Asset(
                                            idx.and_then(|i| external_assets.get(i).cloned()),
                                        ),
                                        v => TemplateValue::Value(v.clone()),
                                    };
                                    fields.push((prop.to_string(), value));
                                }
                            }
                            chunk_bricks
                                .get_mut(brick_index as usize)
                                .ok_or("component brick index out of range")?
                                .components
                                .push(TemplateComponent {
                                    type_name: type_name.clone(),
                                    fields,
                                });
                        }
                    }

                    for (i, &brick_index) in csoa.joint_brick_indices.iter().enumerate() {
                        if gid != 1 {
                            return Err("joint outside the prefab main grid".into());
                        }
                        joints.push((
                            main_bricks.len() + brick_index as usize,
                            csoa.joint_entity_references[i],
                            csoa.joint_initial_relative_offsets[i],
                            csoa.joint_initial_relative_rotations[i],
                        ));
                    }
                }

                if chunk.num_wires > 0 {
                    let wire_soa = db.wire_chunk_soa(gid, chunk.index)?;
                    let local_sources: Vec<LocalWirePortSource> =
                        wire_soa.prop("LocalWireSources")?.try_into()?;
                    let local_targets: Vec<WirePortTarget> =
                        wire_soa.prop("LocalWireTargets")?.try_into()?;
                    let remote_sources: Vec<RemoteWirePortSource> =
                        wire_soa.prop("RemoteWireSources")?.try_into()?;
                    let remote_targets: Vec<WirePortTarget> =
                        wire_soa.prop("RemoteWireTargets")?.try_into()?;

                    // Wire endpoints are resolved to BrickRefs later, once all
                    // bricks are registered; store raw locations for now.
                    for (s, t) in local_sources.iter().zip(&local_targets) {
                        raw_wires.push(RawWire {
                            source: (
                                (gid, chunk_key, s.brick_index_in_chunk),
                                component_type_name(s.component_type_index)?,
                                port_name(s.port_index)?,
                            ),
                            target: (
                                (gid, chunk_key, t.brick_index_in_chunk),
                                component_type_name(t.component_type_index)?,
                                port_name(t.port_index)?,
                            ),
                        });
                    }
                    for (s, t) in remote_sources.iter().zip(&remote_targets) {
                        let s_chunk = (s.chunk_index.x, s.chunk_index.y, s.chunk_index.z);
                        raw_wires.push(RawWire {
                            source: (
                                (
                                    s.grid_persistent_index as usize,
                                    s_chunk,
                                    s.brick_index_in_chunk,
                                ),
                                component_type_name(s.component_type_index)?,
                                port_name(s.port_index)?,
                            ),
                            target: (
                                (gid, chunk_key, t.brick_index_in_chunk),
                                component_type_name(t.component_type_index)?,
                                port_name(t.port_index)?,
                            ),
                        });
                    }
                }

                // Register chunk bricks under their final indices.
                for (i, brick) in chunk_bricks.into_iter().enumerate() {
                    let brick_ref = if gid == 1 {
                        BrickRef::Main(main_bricks.len())
                    } else {
                        let slot = grid_slots[&gid];
                        BrickRef::Grid {
                            grid: slot,
                            brick: grids[slot].bricks.len(),
                        }
                    };
                    brick_refs.insert((gid, chunk_key, i as u32), brick_ref);
                    match brick_ref {
                        BrickRef::Main(_) => main_bricks.push(brick),
                        BrickRef::Grid { grid, .. } => grids[grid].bricks.push(brick),
                    }
                }
            }
        }

        // Re-center main-grid bricks on the prefab's bounding-box center so
        // instances stamp relative to the Blockland brick's position.
        let anchor = anchor_of(&main_bricks).ok_or("prefab has no main-grid bricks")?;
        for brick in &mut main_bricks {
            brick.position = brick.position - anchor;
        }

        let wires = raw_wires
            .into_iter()
            .map(|w| resolve_wire(w, &brick_refs))
            .collect::<Result<Vec<_>, _>>()?;

        let joints = joints
            .into_iter()
            .map(|(main_brick, entity_ref, offset, rotation)| {
                let grid = *grid_slots
                    .get(&(entity_ref as usize))
                    .ok_or("joint references unknown grid entity")?;
                Ok::<_, Box<dyn std::error::Error>>(TemplateJoint {
                    main_brick,
                    grid,
                    relative_offset: offset,
                    relative_rotation: rotation,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            main_bricks,
            grids,
            wires,
            joints,
            angle_offset: 0,
        })
    }
}

/// Per-instance styling taken from the Blockland brick being converted.
pub struct InstanceStyle {
    pub color: brdb::Color,
    pub material: BString,
    pub material_intensity: u8,
    pub collision: bool,
    pub visible: bool,
}

impl PrefabTemplate {
    /// Stamp this template into `world` centered on `center`, rotated to the
    /// Blockland brick's `angle`. Main-grid bricks land on the world's main
    /// grid; each prefab sub-grid becomes a fresh dynamic grid entity, jointed
    /// to its instance's joint brick.
    pub fn instantiate(&self, world: &mut World, center: Position, angle: u8, style: &InstanceStyle) {
        let k = (angle + self.angle_offset) % 4;

        // Register the prefab's external asset references (e.g. interact
        // sounds) in the target world and note their indices there, so
        // component asset fields can be rewritten.
        let mut asset_indices: HashMap<(String, String), usize> = HashMap::new();
        for brick in self
            .main_bricks
            .iter()
            .chain(self.grids.iter().flat_map(|g| g.bricks.iter()))
        {
            for component in &brick.components {
                for (_, value) in &component.fields {
                    if let TemplateValue::Asset(Some(asset)) = value {
                        let (index, _) = world
                            .global_data
                            .external_asset_references
                            .insert_full(asset.clone());
                        world
                            .global_data
                            .external_asset_types
                            .insert(asset.0.clone());
                        asset_indices.insert(asset.clone(), index);
                    }
                }
            }
        }

        let main_ids: Vec<usize> = self.main_bricks.iter().map(|_| Brick::next_id()).collect();
        for (brick, &id) in self.main_bricks.iter().zip(&main_ids) {
            let rel = rotate_position(brick.position, k);
            let (rotation, direction) = compose_z(brick.rotation, brick.direction, k);
            world.add_brick(build_brick(
                brick,
                id,
                center + rel,
                rotation,
                direction,
                style,
                &asset_indices,
            ));
        }

        let grid_entity_ids: Vec<usize> = self.grids.iter().map(|_| Brick::next_id()).collect();
        let mut grid_brick_ids: Vec<Vec<usize>> = Vec::with_capacity(self.grids.len());
        for (grid_slot, (grid, &entity_id)) in
            self.grids.iter().zip(&grid_entity_ids).enumerate()
        {
            let ids: Vec<usize> = grid.bricks.iter().map(|_| Brick::next_id()).collect();
            let bricks: Vec<Brick> = grid
                .bricks
                .iter()
                .zip(&ids)
                .map(|(brick, &id)| {
                    build_brick(
                        brick,
                        id,
                        brick.position,
                        brick.rotation,
                        brick.direction,
                        style,
                        &asset_indices,
                    )
                })
                .collect();
            grid_brick_ids.push(ids);

            // The dynamic grid rides its joint: its world transform is the
            // joint brick's instance transform composed with the joint's
            // stored relative transform.
            let (location, rotation) = match self.joints.iter().find(|j| j.grid == grid_slot) {
                Some(joint) => {
                    let jb = &self.main_bricks[joint.main_brick];
                    let (jrot, _) = compose_z(jb.rotation, jb.direction, k);
                    let jpos = center + rotate_position(jb.position, k);
                    let q_joint = quat_z(jrot);
                    let location = Vector3f {
                        x: jpos.x as f32,
                        y: jpos.y as f32,
                        z: jpos.z as f32,
                    } + quat_rotate(q_joint, joint.relative_offset);
                    (location, quat_mul(q_joint, joint.relative_rotation))
                }
                None => (
                    Vector3f {
                        x: center.x as f32,
                        y: center.y as f32,
                        z: center.z as f32,
                    },
                    Quat4f::default(),
                ),
            };

            world.add_brick_grid(
                Entity {
                    id: Some(entity_id),
                    owner_index: Some(0),
                    location,
                    rotation,
                    color_and_alpha: grid.colors.clone(),
                    ..Default::default()
                },
                bricks,
            );
        }

        let id_of = |r: BrickRef| -> usize {
            match r {
                BrickRef::Main(i) => main_ids[i],
                BrickRef::Grid { grid, brick } => grid_brick_ids[grid][brick],
            }
        };

        for wire in &self.wires {
            world.add_wire_connection(
                WirePort::new(
                    id_of(wire.source.brick),
                    wire.source.component_type.clone(),
                    wire.source.port.clone(),
                ),
                WirePort::new(
                    id_of(wire.target.brick),
                    wire.target.component_type.clone(),
                    wire.target.port.clone(),
                ),
            );
        }

        for joint in &self.joints {
            world.register_joint_link(
                main_ids[joint.main_brick],
                grid_entity_ids[joint.grid],
                joint.relative_offset,
                joint.relative_rotation,
            );
        }
    }

}

fn build_brick(
    template: &TemplateBrick,
    id: usize,
    position: Position,
    rotation: Rotation,
    direction: Direction,
    style: &InstanceStyle,
    asset_indices: &HashMap<(String, String), usize>,
) -> Brick {
    let collision = if style.collision {
        template.collision.clone()
    } else {
        Collision {
            player: false,
            player1: Some(false),
            player2: Some(false),
            player3: Some(false),
            weapon: false,
            interact: false,
            tool: false,
            physics: false,
        }
    };
    Brick {
        id: Some(id),
        asset: template.asset.clone(),
        owner_index: Some(0),
        original_owner_index: None,
        position,
        rotation,
        direction,
        collision,
        visible: template.visible && style.visible,
        color: style.color,
        material: style.material.clone(),
        material_intensity: style.material_intensity,
        components: template
            .components
            .iter()
            .map(|c| Box::new(c.to_literal(asset_indices)) as Box<dyn brdb::BrdbComponent>)
            .collect(),
    }
}

impl TemplateComponent {
    fn to_literal(&self, asset_indices: &HashMap<(String, String), usize>) -> LiteralComponent {
        LiteralComponent::new(self.type_name.clone()).with_data(self.fields.iter().map(
            |(name, value)| {
                let value: Box<dyn AsBrdbValue> = match value {
                    TemplateValue::Value(v) => Box::new(v.clone()),
                    TemplateValue::Asset(asset) => Box::new(BrdbValue::Asset(
                        asset
                            .as_ref()
                            .and_then(|asset| asset_indices.get(asset).copied()),
                    )),
                };
                (name.clone(), value)
            },
        ))
    }
}

/// Rotate a template-relative position `k` 90° steps about Z, matching the
/// converter's `rotate_offset` convention for mapping offsets.
fn rotate_position(p: Position, k: u8) -> Position {
    let (mut x, mut y) = (p.x, p.y);
    for _ in 0..k % 4 {
        (x, y) = (-y, x);
    }
    Position::new(x, y, p.z)
}

/// Compose a brick's (rotation, direction) with `k` extra 90° steps about
/// world Z. Up/down-facing bricks spin in place; sideways-facing bricks have
/// their facing axis rotated instead.
fn compose_z(rotation: Rotation, direction: Direction, k: u8) -> (Rotation, Direction) {
    let k = k % 4;
    let rot = rotation as u8;
    match direction {
        Direction::ZPositive => (rotation_from(rot + k), direction),
        Direction::ZNegative => (rotation_from(rot + 4 - k), direction),
        _ => {
            let cycle = [
                Direction::XPositive,
                Direction::YPositive,
                Direction::XNegative,
                Direction::YNegative,
            ];
            let i = match direction {
                Direction::XPositive => 0,
                Direction::YPositive => 1,
                Direction::XNegative => 2,
                _ => 3,
            };
            (rotation, cycle[(i + k as usize) % 4])
        }
    }
}

fn rotation_from(steps: u8) -> Rotation {
    match steps % 4 {
        0 => Rotation::Deg0,
        1 => Rotation::Deg90,
        2 => Rotation::Deg180,
        _ => Rotation::Deg270,
    }
}

/// Quaternion for a brick rotation about +Z, matching the same 90°-step
/// convention as `rotate_position`.
fn quat_z(rotation: Rotation) -> Quat4f {
    let half = (rotation as u8 as f32) * std::f32::consts::FRAC_PI_4;
    Quat4f {
        x: 0.0,
        y: 0.0,
        z: half.sin(),
        w: half.cos(),
    }
}

fn quat_mul(a: Quat4f, b: Quat4f) -> Quat4f {
    Quat4f {
        x: a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        y: a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        z: a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        w: a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    }
}

fn quat_rotate(q: Quat4f, v: Vector3f) -> Vector3f {
    // v' = q * (v, 0) * q⁻¹, expanded via t = 2 (q.xyz × v)
    let (qx, qy, qz, qw) = (q.x, q.y, q.z, q.w);
    let tx = 2.0 * (qy * v.z - qz * v.y);
    let ty = 2.0 * (qz * v.x - qx * v.z);
    let tz = 2.0 * (qx * v.y - qy * v.x);
    Vector3f {
        x: v.x + qw * tx + (qy * tz - qz * ty),
        y: v.y + qw * ty + (qz * tx - qx * tz),
        z: v.z + qw * tz + (qx * ty - qy * tx),
    }
}

/// Raw wire endpoint before bricks are assigned template indices:
/// `((grid_id, chunk, brick_index_in_chunk), component_type, port)`.
type RawWireEnd = ((usize, (i16, i16, i16), u32), String, String);

struct RawWire {
    source: RawWireEnd,
    target: RawWireEnd,
}

fn resolve_wire(
    wire: RawWire,
    brick_refs: &HashMap<(usize, (i16, i16, i16), u32), BrickRef>,
) -> Result<TemplateWire, Box<dyn std::error::Error>> {
    let resolve = |(loc, component_type, port): RawWireEnd| {
        brick_refs
            .get(&loc)
            .copied()
            .map(|brick| TemplateWireEnd {
                brick,
                component_type,
                port,
            })
            .ok_or_else(|| format!("wire references unknown brick {loc:?}"))
    };
    Ok(TemplateWire {
        source: resolve(wire.source)?,
        target: resolve(wire.target)?,
    })
}

fn anchor_of(bricks: &[TemplateBrick]) -> Option<Position> {
    let mut min = Position::new(i32::MAX, i32::MAX, i32::MAX);
    let mut max = Position::new(i32::MIN, i32::MIN, i32::MIN);
    for b in bricks {
        // Procedural bricks contribute their world-axis-aligned extents; the
        // stored size is pre-rotation, so swap x/y for 90°/270° spins. Basic
        // bricks contribute their center only.
        let (hx, hy, hz) = match (&b.asset, b.direction) {
            (BrickType::Procedural { size, .. }, Direction::ZPositive | Direction::ZNegative) => {
                let (x, y) = match b.rotation {
                    Rotation::Deg90 | Rotation::Deg270 => (size.y as i32, size.x as i32),
                    _ => (size.x as i32, size.y as i32),
                };
                (x, y, size.z as i32)
            }
            _ => (0, 0, 0),
        };
        min.x = min.x.min(b.position.x - hx);
        min.y = min.y.min(b.position.y - hy);
        min.z = min.z.min(b.position.z - hz);
        max.x = max.x.max(b.position.x + hx);
        max.y = max.y.max(b.position.y + hy);
        max.z = max.z.max(b.position.z + hz);
    }
    if bricks.is_empty() {
        return None;
    }
    Some(Position::new(
        (min.x + max.x) / 2,
        (min.y + max.y) / 2,
        (min.z + max.z) / 2,
    ))
}
