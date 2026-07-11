//! Blockland `+-LIGHT` datablocks translated into Brickadia `Component_PointLight`
//! brick components.
//!
//! Blockland lights are simple point lights that hang off a brick. We map the
//! handful of stock datablocks we recognize onto a point light attached to the
//! converted brick. These lights never cast shadows, never match the brick's
//! shape, and carry their own color rather than inheriting the brick's.

use brdb::assets::LiteralComponent;
use brdb::{AsBrdbValue, SavedBrickColor};

/// Brickadia stores positions and radii in centimeter-scale units where one
/// stud is ten units, so a stud radius scales up by this factor.
const CM_PER_STUD: f32 = 10.0;

/// A resolved point light: its color plus brightness (lumens) and radius (studs).
pub struct LightSpec {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub brightness: f32,
    pub radius_studs: f32,
}

/// The two Blockland light "sizes": a focused light and a dim, wide ambient one.
/// Every stock datablock we support is one of these in some color.
const FOCUSED_LM: f32 = 200.0;
const FOCUSED_ST: f32 = 3.0;
const AMBIENT_LM: f32 = 100.0;
const AMBIENT_ST: f32 = 20.0;

/// Map a Blockland light datablock `uiName` to a point light, or `None` if we
/// don't support that datablock yet.
pub fn map_light(name: &str) -> Option<LightSpec> {
    let focused = |r, g, b| {
        Some(LightSpec {
            r,
            g,
            b,
            brightness: FOCUSED_LM,
            radius_studs: FOCUSED_ST,
        })
    };
    let ambient = |r, g, b| {
        Some(LightSpec {
            r,
            g,
            b,
            brightness: AMBIENT_LM,
            radius_studs: AMBIENT_ST,
        })
    };

    match name {
        "Player's Light" => focused(255, 255, 255),

        "Orange Light" => focused(255, 127, 0),
        "Yellow Light" => focused(255, 255, 0),
        "Red Light" => focused(255, 0, 0),
        "Blue Light" => focused(0, 0, 255),
        "Cyan Light" => focused(0, 255, 255),
        "Green Light" => focused(0, 255, 0),
        "Purple Light" => focused(128, 0, 255),

        "Orange Amb. Dimmer" => ambient(255, 127, 0),
        "Yellow Amb. Dimmer" => ambient(255, 255, 0),
        "Red Amb. Dimmer" => ambient(255, 0, 0),
        "Blue Amb. Dimmer" => ambient(0, 0, 255),
        "Cyan Amb. Dimmer" => ambient(0, 255, 255),
        "Green Amb. Dimmer" => ambient(0, 255, 0),
        "Purple Amb. Dimmer" => ambient(128, 0, 255),

        _ => None,
    }
}

/// Build the `Component_PointLight` component for a resolved light. The
/// component's schema and defaults come from brdb's built-in component
/// database; only the values that differ from those defaults are set here.
pub fn point_light_component(spec: &LightSpec) -> LiteralComponent {
    LiteralComponent::new("Component_PointLight").with_data([
        (
            "bMatchBrickShape",
            Box::new(false) as Box<dyn AsBrdbValue>,
        ),
        ("bEnabled", Box::new(true)),
        ("Brightness", Box::new(spec.brightness)),
        ("Radius", Box::new(spec.radius_studs * CM_PER_STUD)),
        (
            "Color",
            Box::new(SavedBrickColor::rgb(spec.r, spec.g, spec.b)),
        ),
        ("bUseBrickColor", Box::new(false)),
        ("bCastShadows", Box::new(false)),
    ])
}
