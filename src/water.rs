//! Blockland water bricks translated into Brickadia water zones.
//!
//! Brickadia's QA water is an invisible `BP_ZoneProjector` with an attached
//! zone component. The projector sits at the bottom of the water volume; the
//! component, rather than the brick's physical height, defines its 72-unit
//! height.

use brdb::assets::LiteralComponent;
use brdb::schema::{BrdbInterned, BrdbSchema, BrdbStructPropRaw};
use brdb::{
    AsBrdbValue, BrdbSchemaError, Brick, BrickSize, BrickType, Collision, Color, Direction,
    Position, Rotation,
};

/// Blockland's water shape-FX value.
const SHAPE_FX_WATER: u8 = 2;
const UNITS_PER_BLOCKLAND_STUD: u32 = 5;
const ZONE_HEIGHT: i32 = 72;
/// Blockland stores the water brick at the center of its 72-unit volume. The
/// two-unit-tall projector is centered on its physical brick, so move from the
/// source center down to the water bottom, then up to that brick's center.
const PROJECTOR_CENTER_OFFSET: i32 = PROJECTOR_CENTER_ABOVE_BOTTOM - ZONE_HEIGHT / 2 - 1;
const PROJECTOR_CENTER_ABOVE_BOTTOM: i32 = 2;

/// Return the water-zone projector dimensions for a stock water brick.
///
/// `8x Water` is square; `1x1 Water`-style names are handled as rectangular
/// water zones too. Only bricks actually carrying Blockland's water shape FX
/// are converted, avoiding accidental conversion of add-on bricks named Water.
fn dimensions(from: &bls::Brick) -> Option<(u32, u32)> {
    if from.shape_fx != SHAPE_FX_WATER {
        return None;
    }

    let name = from.name.strip_suffix(" Water")?;
    let (width, length): (u32, u32) = if let Some(square) = name.strip_suffix('x') {
        let side: u32 = square.parse().ok()?;
        (side, side)
    } else {
        let (width, length) = name.split_once('x')?;
        (width.parse().ok()?, length.parse().ok()?)
    };
    if width == 0 || length == 0 {
        return None;
    }
    Some((
        width * UNITS_PER_BLOCKLAND_STUD,
        length * UNITS_PER_BLOCKLAND_STUD,
    ))
}

/// Build a Brickadia water zone for a stock Blockland water brick.
pub fn zone(from: &bls::Brick, owner_index: usize) -> Option<Brick> {
    let (width, length) = dimensions(from)?;
    let mut brick = Brick {
        id: None,
        asset: BrickType::Procedural {
            asset: "BP_ZoneProjector".into(),
            size: BrickSize::new(width as u16, length as u16, 2),
        },
        owner_index: Some(owner_index),
        original_owner_index: None,
        position: Position {
            x: (from.position.1 * 20.0) as i32,
            y: (from.position.0 * 20.0) as i32,
            z: (from.position.2 * 20.0) as i32 + PROJECTOR_CENTER_OFFSET,
        },
        rotation: Rotation::Deg0,
        direction: Direction::ZPositive,
        collision: Collision {
            player: false,
            player1: Some(false),
            player2: Some(false),
            player3: Some(false),
            weapon: false,
            interact: false,
            tool: false,
            physics: false,
        },
        visible: false,
        color: Color::new(255, 255, 255),
        material: "BMC_Plastic".into(),
        material_intensity: 5,
        components: Vec::new(),
    };
    brick.add_component(water_component());
    Some(brick)
}

fn water_component() -> LiteralComponent {
    LiteralComponent::new("Component_Internal_AttachedZone").with_data(vec![
        ("ZoneStartDistance", Box::new(0i32) as Box<dyn AsBrdbValue>),
        ("ZoneEndDistance", Box::new(ZONE_HEIGHT)),
        ("bIsBuildingZone", Box::new(false)),
        ("bIsLooseZone", Box::new(false)),
        ("bIsShareZone", Box::new(false)),
        ("bWater", Box::new(true)),
        (
            "WaterScattering",
            Box::new(Vector::new(0.0005, 0.0005, 0.0005)),
        ),
        (
            "WaterAbsorption",
            Box::new(Vector::new(0.0125, 0.000833, 0.000357)),
        ),
        ("WaterFogIntensity", Box::new(0.0002f32)),
        (
            "WaterFogScatteringColor",
            Box::new(LinearColor::new(0.5, 1.0, 0.75, 1.0)),
        ),
        (
            "WaterFogAmbientColor",
            Box::new(LinearColor::new(0.1, 0.409524, 0.75, 1.0)),
        ),
        ("WaterFogScatteringScale", Box::new(3.0f32)),
        ("WaterFogAmbientScale", Box::new(0.25f32)),
        ("bDetectPlayers", Box::new(true)),
        ("bDetectPlayers1", Box::new(true)),
        ("bDetectPlayers2", Box::new(true)),
        ("bDetectPlayers3", Box::new(true)),
        ("bDetectEntities", Box::new(false)),
        ("bDetectProjectiles", Box::new(false)),
    ])
}

/// The bundled brdb component catalog predates QA water. Replace its ordinary
/// attached-zone definition with the QA water-zone schema after it registers
/// the component type table.
pub fn register_schema(schema: &mut BrdbSchema) {
    let fields = |fields: &[(&str, &str)]| {
        fields
            .iter()
            .map(|(name, ty)| {
                (
                    (*name).to_string(),
                    BrdbStructPropRaw::Type((*ty).to_string()),
                )
            })
            .collect()
    };
    schema.add_struct(
        "LinearColor".to_string(),
        fields(&[("R", "f32"), ("G", "f32"), ("B", "f32"), ("A", "f32")]),
    );
    schema.add_struct(
        "Vector".to_string(),
        fields(&[("X", "f64"), ("Y", "f64"), ("Z", "f64")]),
    );
    schema.add_struct(
        "BrickComponentData_Internal_AttachedZone".to_string(),
        fields(&[
            ("ZoneStartDistance", "i32"),
            ("ZoneEndDistance", "i32"),
            ("bIsBuildingZone", "bool"),
            ("bIsLooseZone", "bool"),
            ("bIsShareZone", "bool"),
            ("bWater", "bool"),
            ("WaterScattering", "Vector"),
            ("WaterAbsorption", "Vector"),
            ("WaterFogIntensity", "f32"),
            ("WaterFogScatteringColor", "LinearColor"),
            ("WaterFogAmbientColor", "LinearColor"),
            ("WaterFogScatteringScale", "f32"),
            ("WaterFogAmbientScale", "f32"),
            ("bDetectPlayers", "bool"),
            ("bDetectPlayers1", "bool"),
            ("bDetectPlayers2", "bool"),
            ("bDetectPlayers3", "bool"),
            ("bDetectEntities", "bool"),
            ("bDetectProjectiles", "bool"),
        ]),
    );
}

#[derive(Clone, Copy)]
struct Vector {
    x: f64,
    y: f64,
    z: f64,
}

impl Vector {
    const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl AsBrdbValue for Vector {
    fn as_brdb_struct_prop_value(
        &self,
        schema: &BrdbSchema,
        _: BrdbInterned,
        prop: BrdbInterned,
    ) -> Result<&dyn AsBrdbValue, BrdbSchemaError> {
        match prop.get(schema).unwrap() {
            "X" => Ok(&self.x),
            "Y" => Ok(&self.y),
            "Z" => Ok(&self.z),
            field => unreachable!("unknown Vector field {field}"),
        }
    }
}

#[derive(Clone, Copy)]
struct LinearColor {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl LinearColor {
    const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

impl AsBrdbValue for LinearColor {
    fn as_brdb_struct_prop_value(
        &self,
        schema: &BrdbSchema,
        _: BrdbInterned,
        prop: BrdbInterned,
    ) -> Result<&dyn AsBrdbValue, BrdbSchemaError> {
        match prop.get(schema).unwrap() {
            "R" => Ok(&self.r),
            "G" => Ok(&self.g),
            "B" => Ok(&self.b),
            "A" => Ok(&self.a),
            field => unreachable!("unknown LinearColor field {field}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brdb::schema::BrdbValue;
    use brdb::{Brz, IntoReader};

    fn water(name: &str, shape_fx: u8) -> bls::Brick {
        let mut brick = bls::Brick::new(name);
        brick.shape_fx = shape_fx;
        brick.position = (4.0, -3.0, 6.0);
        brick
    }

    #[test]
    fn eight_by_water_is_a_40_unit_square_zone() {
        let zone = zone(&water("8x Water", SHAPE_FX_WATER), 0).unwrap();
        let BrickType::Procedural { size, .. } = zone.asset else {
            panic!("not procedural")
        };
        assert_eq!((size.x, size.y, size.z), (40, 40, 2));
        assert_eq!(
            zone.position,
            Position {
                x: -60,
                y: 80,
                z: 85
            }
        );
    }

    #[test]
    fn thirty_two_by_water_keeps_the_72_unit_water_height() {
        let component = water_component();
        let zone = zone(&water("32x Water", SHAPE_FX_WATER), 0).unwrap();
        let BrickType::Procedural { size, .. } = zone.asset else {
            panic!("not procedural")
        };
        assert_eq!((size.x, size.y, size.z), (160, 160, 2));
        assert_eq!(
            component.component_name.as_ref(),
            "Component_Internal_AttachedZone"
        );
    }

    #[test]
    fn water_named_bricks_without_water_shape_fx_are_not_zones() {
        assert!(zone(&water("8x Water", 0), 0).is_none());
    }

    #[test]
    fn converted_water_zone_serializes_with_its_qa_schema() {
        let mut save = bls::Save::new();
        save.bricks.push(water("8x Water", SHAPE_FX_WATER));
        save.bricks.push(bls::Brick::new("1x1"));

        let converted = crate::convert(&save);
        assert_eq!(converted.world.bricks.len(), 1);
        assert_eq!(converted.world.grids.len(), 1);
        assert_eq!(converted.world.grids[0].1.len(), 1);
        let bytes = converted
            .world
            .to_brz_vec()
            .expect("water-zone save should serialize");
        let db = Brz::read_slice(&bytes).unwrap().into_reader();
        let chunk = db.brick_chunk_index(1).unwrap().remove(0);
        let (_, components) = db.component_chunk_soa(1, chunk.index).unwrap();
        assert!(components
            .iter()
            .any(|component| { matches!(component.get("bWater"), Some(BrdbValue::Bool(true))) }));
    }
}
