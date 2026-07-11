//! Blockland letter prints (`Letters/A`, `Letters/B`, ...) translated into
//! Brickadia `Component_TextDisplay` text decals.
//!
//! A Blockland print is a texture stamped onto one face of a brick. The stock
//! `Letters/*` prints are single glyphs, so we drop the texture and instead
//! attach a text decal carrying that glyph. Brickadia renders the decal with a
//! real font (`RobotoMono`) rather than a baked image.
//!
//! Placement here is deliberately minimal — the decal faces one fixed direction
//! at a fixed size, tuned for the common 1x1 print brick. Larger print bricks
//! will need their own face/size handling later.

use brdb::assets::LiteralComponent;
use brdb::schema::{BrdbInterned, BrdbSchema, BrdbValue};
use brdb::{AsBrdbValue, BrdbSchemaError, SavedBrickColor};

/// The font descriptor the decals reference. Brickadia resolves this by name
/// through the world's external asset reference table (populated in `lib.rs`).
pub const FONT_ASSET_TYPE: &str = "BrickFontDescriptor";
pub const FONT_ASSET_NAME: &str = "RobotoMono";

/// Text height for the common 1x1 print brick.
const LETTER_SIZE: f32 = 8.0;

/// `EBrickDirection::X_Negative` — the face the 1x1 letter print renders on.
const FACE_X_NEGATIVE: u8 = 1;

/// `EBRTextMaterial::Unlit`.
const MATERIAL_UNLIT: u8 = 0;

/// `EBRTextOutline::Outlined` — a plain outline around the glyph.
const OUTLINE_OUTLINED: u8 = 2;
const OUTLINE_WIDTH: f32 = 2.0;

/// Center the glyph on the brick face.
const ANCHOR_CENTER: f32 = 0.5;

/// If `print` is a stock letter print (`Letters/A`), return the glyph to show.
pub fn letter_from_print(print: &str) -> Option<&str> {
    let letter = print.strip_prefix("Letters/")?;
    (!letter.is_empty()).then_some(letter)
}

/// Build a `Component_TextDisplay` decal showing `text`, tuned for the 1x1
/// letter print brick: a single `RobotoMono` glyph, unlit, on the `-X` face.
///
/// The set fields mirror the game's own text-component defaults (centered
/// anchor, white glyph, enabled outline) plus the print-specific choices
/// (font, size, face, material); the writer fills any remaining field from the
/// component's built-in defaults. `Font` is asset index 0, which `lib.rs`
/// registers as the RobotoMono font descriptor.
pub fn text_decal_component(text: &str) -> LiteralComponent {
    LiteralComponent::new("Component_TextDisplay").with_data([
        ("Text", Box::new(text.to_string()) as Box<dyn AsBrdbValue>),
        ("Font", Box::new(BrdbValue::Asset(Some(0)))),
        ("LineHeight", Box::new(LETTER_SIZE)),
        ("Face", Box::new(FACE_X_NEGATIVE)),
        ("Material", Box::new(MATERIAL_UNLIT)),
        (
            "Anchor",
            Box::new(Vec2f {
                x: ANCHOR_CENTER,
                y: ANCHOR_CENTER,
            }),
        ),
        ("Color", Box::new(SavedBrickColor::rgb(255, 255, 255))),
        ("bOverrideColor", Box::new(true)),
        ("Outline", Box::new(OUTLINE_OUTLINED)),
        ("OutlineWidth", Box::new(OUTLINE_WIDTH)),
    ])
}

/// A two-component float vector (`Vector2f`), whose `X`/`Y` fields the writer
/// reads by name. brdb has no public helper for this component type.
struct Vec2f {
    x: f32,
    y: f32,
}

impl AsBrdbValue for Vec2f {
    fn as_brdb_struct_prop_value(
        &self,
        schema: &BrdbSchema,
        _struct_name: BrdbInterned,
        prop_name: BrdbInterned,
    ) -> Result<&dyn AsBrdbValue, BrdbSchemaError> {
        match prop_name.get(schema).unwrap() {
            "X" => Ok(&self.x),
            "Y" => Ok(&self.y),
            other => Err(BrdbSchemaError::MissingStructField(
                "Vector2f".to_string(),
                other.to_string(),
            )),
        }
    }
}
