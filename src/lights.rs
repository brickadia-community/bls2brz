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

/// Blockland lights come in two broad "sizes": a focused light and a dim, wide
/// ambient one, each in some color. A few named lights get their own tuning.
const FOCUSED_LM: f32 = 30.0;
const FOCUSED_ST: f32 = 3.0;
const AMBIENT_LM: f32 = 100.0;
const AMBIENT_ST: f32 = 20.0;

/// A small warm candle flame.
const CANDLE_LM: f32 = 10.0;
const CANDLE_ST: f32 = 1.0;
const CANDLE_RGB: (u8, u8, u8) = (255, 147, 41);

/// The stronger "Bright" white light.
const BRIGHT_LM: f32 = 60.0;
const BRIGHT_ST: f32 = 4.0;

/// Warm-white incandescent bulb color.
const BULB_RGB: (u8, u8, u8) = (255, 244, 214);

/// Animated/utility lights we don't reproduce (RGB cyclers, alarms, strobes,
/// blinkers) plus custom image-projector datablocks (`Eksi ...`).
const SKIP_MARKERS: &[&str] = &["RGB", "Alarm", "Strobe", "Blink", "Eksi"];

/// Map a Blockland light datablock `uiName` to a point light, or `None` if we
/// don't support that datablock.
///
/// Color is taken from the color word in the name; the tier (focused vs wide
/// ambient) from whether the name mentions "Amb"/"Ambient". A handful of named
/// lights (candle, player, bright, bulb) get bespoke tuning.
pub fn map_light(name: &str) -> Option<LightSpec> {
    if SKIP_MARKERS.iter().any(|s| name.contains(s)) {
        return None;
    }

    let spec = |(r, g, b), brightness, radius_studs| LightSpec {
        r,
        g,
        b,
        brightness,
        radius_studs,
    };
    let focused = |rgb| spec(rgb, FOCUSED_LM, FOCUSED_ST);
    let ambient = |rgb| spec(rgb, AMBIENT_LM, AMBIENT_ST);

    // "Candle Light" / "Candlelight Warm": small warm flame.
    if name.contains("Candle") {
        return Some(spec(CANDLE_RGB, CANDLE_LM, CANDLE_ST));
    }
    // Player torch and incandescent bulb are white/warm-white focused lights.
    if name.starts_with("Player") {
        return Some(focused((255, 255, 255)));
    }
    if name == "Bright" {
        return Some(spec((255, 255, 255), BRIGHT_LM, BRIGHT_ST));
    }
    if name.starts_with("Bulb") {
        return Some(focused(BULB_RGB));
    }

    let rgb = color_of(name)?;
    if name.contains("Amb") {
        Some(ambient(rgb))
    } else {
        Some(focused(rgb))
    }
}

/// Detect the color word in a light datablock name. `None` for names with no
/// recognized color (which are then left unconverted).
fn color_of(name: &str) -> Option<(u8, u8, u8)> {
    let n = name.to_ascii_lowercase();
    let has = |word: &str| n.contains(word);
    Some(if has("orange") {
        (255, 127, 0)
    } else if has("yellow") {
        (255, 255, 0)
    } else if has("red") {
        (255, 0, 0)
    } else if has("blue") {
        (0, 0, 255)
    } else if has("cyan") {
        (0, 255, 255)
    } else if has("green") {
        (0, 255, 0)
    } else if has("purple") {
        (128, 0, 255)
    } else if has("white") {
        (255, 255, 255)
    } else {
        return None;
    })
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

#[cfg(test)]
mod tests {
    use super::map_light;

    #[test]
    fn skips_specials_maps_regulars() {
        for n in ["RGB","Alarm","Strobe","Yellow Blink","Eksi (IMG) - Diag yellow","Eksi (AMB) 1/2 - Neutral star"] {
            assert!(map_light(n).is_none(), "should skip {n}");
        }
        for n in ["Candle Light","Candlelight Warm","Player's Light","Players Light Dimmer","Bright","Bulb Light","Bulb Light Dimmer","Orange Light","Orange Warm","Orange Amb. Dim","White Ambient","Cyan Ambient Dimmer","Green Amb.","Red Warm"] {
            assert!(map_light(n).is_some(), "should map {n}");
        }
    }

    #[test]
    fn candle_is_warm_small_dim() {
        let c = map_light("Candle Light").unwrap();
        assert_eq!((c.r, c.g, c.b), (255, 147, 41));
        assert_eq!(c.brightness, 10.0);
        assert_eq!(c.radius_studs, 1.0);
    }

    #[test]
    fn focused_is_30lm() {
        assert_eq!(map_light("Orange Light").unwrap().brightness, 30.0);
    }
}
