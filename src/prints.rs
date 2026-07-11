//! Blockland letter prints (`Letters/A`, `Letters/B`, ...) translated into
//! Brickadia `Component_TextDisplay` text decals.
//!
//! A Blockland print is a texture stamped onto one face of a brick. The stock
//! `Letters/*` prints are single glyphs, so we drop the texture and instead
//! attach a text decal carrying that glyph. Brickadia renders the decal with a
//! real font (`RobotoMono`) rather than a baked image.
//!
//! Placement (glyph size and which face) is derived from the converted brick's
//! geometry in [`print_placement`]: flat bricks take the print on top, upright
//! bricks on the side, with the glyph scaled to fill the face. Irregular shapes
//! (wedges, rounds, poles, corners, vertical-edge prints, ...) have bespoke
//! print spots the generic rule can't reproduce, so [`is_regular_print_brick`]
//! filters them out and they get no decal.

use brdb::assets::LiteralComponent;
use brdb::schema::{BrdbInterned, BrdbSchema, BrdbValue};
use brdb::{AsBrdbValue, BrdbSchemaError, SavedBrickColor};

/// The font descriptor the decals reference. Brickadia resolves this by name
/// through the world's external asset reference table (populated in `lib.rs`).
pub const FONT_ASSET_TYPE: &str = "BrickFontDescriptor";
pub const FONT_ASSET_NAME: &str = "RobotoMono";

/// Text height for a print spanning one stud. Larger print bricks scale up from
/// this (a 2-stud face is `2 * PER_STUD_SIZE`, etc.).
const PER_STUD_SIZE: f32 = 8.0;

/// Converted brick sizes are in 1/5-stud units (5 units == 1 stud).
const UNITS_PER_STUD: f32 = 5.0;

/// `EBrickDirection` discriminants for the faces a print can land on.
const FACE_X_NEGATIVE: u8 = 1; // upright brick, print on the -X side
const FACE_Z_POSITIVE: u8 = 4; // flat brick, print on top
const FACE_Z_NEGATIVE: u8 = 5; // flat ceiling brick, print underneath

/// `EBRTextMaterial::Unlit`.
const MATERIAL_UNLIT: u8 = 0;

/// `EBRTextOutline::Outlined` — a plain outline around the glyph.
const OUTLINE_OUTLINED: u8 = 2;
const OUTLINE_WIDTH: f32 = 2.0;

/// Center the glyph on the brick face.
const ANCHOR_CENTER: f32 = 0.5;

/// If `print` is a stock letter print, return the glyph to show.
///
/// Plain glyph prints (`Letters/A`, `Letters/7`) yield that character directly.
/// Named symbol prints (`Letters/-less_than`) map to their punctuation glyph.
/// Prints with no printable glyph — `Letters/-space`, image icons
/// (`Letters/icon_*`), and any unrecognized symbol name — produce no decal.
pub fn letter_from_print(print: &str) -> Option<&str> {
    let name = print.strip_prefix("Letters/")?;

    if let Some(symbol) = name.strip_prefix('-') {
        return symbol_glyph(symbol);
    }

    // `icon_*` prints are pictographs (weapons, etc.), not letters.
    if name.is_empty() || name.starts_with("icon_") || name.starts_with("Icon_") {
        return None;
    }

    Some(name)
}

/// Map a Blockland named-symbol print (the part after `Letters/-`) to its glyph.
/// `space` and anything unrecognized return `None` so no decal is emitted.
fn symbol_glyph(symbol: &str) -> Option<&'static str> {
    Some(match symbol {
        "space" => return None,
        "apostrophe" => "'",
        "and" => "&",
        "asterisk" => "*",
        "at" => "@",
        "backslash" => "\\",
        "bang" => "!",
        "caret" => "^",
        "colon" => ":",
        "comma" => ",",
        "currencysign" => "¤",
        "dollar" => "$",
        "equals" => "=",
        "greater_than" => ">",
        "less_than" => "<",
        "minus" => "-",
        "percent" => "%",
        "period" => ".",
        "plus" => "+",
        "pound" => "#",
        "qmark" => "?",
        "roundbracketleft" => "(",
        "roundbracketright" => ")",
        "semicolon" => ";",
        "slash" => "/",
        "squarebracketleft" => "[",
        "squarebracketright" => "]",
        "tilde" => "~",
        "underscore" => "_",
        "verticalbar" => "|",
        _ => return None,
    })
}

/// Brick-name markers for print bricks whose print sits on a special face or in
/// a special spot (wedges, rounds, poles, corners, vertical-edge plates, ...).
/// The generic [`print_placement`] rule can't place these, so they get no decal.
const IRREGULAR_MARKERS: &[&str] = &[
    "wedge", "round", "pole", "corner", "edge", "shaped", "barrel", "panel",
    "thirds", "half", "bottom", "sided", "vertical",
];

/// Whether a print brick has a plain rectangular print face the generic
/// placement rule handles. Irregular shapes return `false` and are skipped.
pub fn is_regular_print_brick(brick_name: &str) -> bool {
    let name = brick_name.to_ascii_lowercase();
    !IRREGULAR_MARKERS.iter().any(|m| name.contains(m))
}

/// Where a print sits on its brick: the glyph height and the face it renders on.
/// Derived from the converted brick's size via [`print_placement`].
#[derive(Copy, Clone)]
pub struct PrintPlacement {
    pub size: f32,
    pub face: u8,
}

impl Default for PrintPlacement {
    /// The common 1x1 side print: one-stud glyph on the `-X` face.
    fn default() -> Self {
        Self {
            size: PER_STUD_SIZE,
            face: FACE_X_NEGATIVE,
        }
    }
}

/// Resolve glyph size and face from a converted print brick's size (in 1/5-stud
/// units) and whether it's a ceiling brick.
///
/// A brick flat enough that Z is its smallest dimension takes the print on its
/// top face (`+Z`, or `-Z` underneath for ceiling bricks); anything upright
/// takes it on the `-X` side. The glyph scales to the shorter edge of that face
/// so a single letter fills it: 1 stud -> 8, 2 studs -> 16, 4 studs -> 32.
pub fn print_placement(size: (u32, u32, u32), ceiling: bool) -> PrintPlacement {
    let (sx, sy, sz) = size;
    if sx == 0 || sy == 0 || sz == 0 {
        return PrintPlacement::default();
    }

    let glyph = |face_units: u32| PER_STUD_SIZE * (face_units as f32 / UNITS_PER_STUD);

    if sz <= sx && sz <= sy {
        // Flat brick: print faces up (or down for a ceiling brick).
        PrintPlacement {
            size: glyph(sx.min(sy)),
            face: if ceiling {
                FACE_Z_NEGATIVE
            } else {
                FACE_Z_POSITIVE
            },
        }
    } else {
        // Upright brick: print on the -X side, spanning the Y and Z edges.
        PrintPlacement {
            size: glyph(sy.min(sz)),
            face: FACE_X_NEGATIVE,
        }
    }
}

/// Build a `Component_TextDisplay` decal showing `text`, sized and faced per
/// `placement`: a single `RobotoMono` glyph, unlit.
///
/// The set fields mirror the game's own text-component defaults (centered
/// anchor, white glyph, enabled outline) plus the print-specific choices
/// (font, size, face, material); the writer fills any remaining field from the
/// component's built-in defaults. `Font` is asset index 0, which `lib.rs`
/// registers as the RobotoMono font descriptor.
pub fn text_decal_component(text: &str, placement: PrintPlacement) -> LiteralComponent {
    LiteralComponent::new("Component_TextDisplay").with_data([
        ("Text", Box::new(text.to_string()) as Box<dyn AsBrdbValue>),
        ("Font", Box::new(BrdbValue::Asset(Some(0)))),
        ("LineHeight", Box::new(placement.size)),
        ("Face", Box::new(placement.face)),
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

#[cfg(test)]
mod placement_tests {
    use super::*;

    fn p(size: (u32, u32, u32), ceiling: bool) -> (f32, u8) {
        let pl = print_placement(size, ceiling);
        (pl.size, pl.face)
    }

    #[test]
    fn matches_known_print_bricks() {
        assert_eq!(p((5, 5, 6), false), (8.0, FACE_X_NEGATIVE)); // 1x1 Print
        assert_eq!(p((5, 20, 24), false), (32.0, FACE_X_NEGATIVE)); // 1x4x4 Print
        assert_eq!(p((5, 5, 2), false), (8.0, FACE_Z_POSITIVE)); // 1x1F Print
        assert_eq!(p((10, 10, 2), false), (16.0, FACE_Z_POSITIVE)); // 2x2F Print
        assert_eq!(p((5, 5, 2), true), (8.0, FACE_Z_NEGATIVE)); // ceiling flat
        assert_eq!(p((0, 0, 0), false), (8.0, FACE_X_NEGATIVE)); // fallback
    }
}

#[cfg(test)]
mod regular_tests {
    use super::is_regular_print_brick as reg;

    #[test]
    fn skips_irregular_keeps_boxes() {
        for n in ["1x1 Print","1x4x4 Print","1x1F Print","2x2F Print","4x4f_PrintPlate","2x2x1 Print","2x Cube Print","2x2F Ceiling Print Plate"] {
            assert!(reg(n), "should keep {n}");
        }
        for n in ["1x1 Wedge Print","2x2F Round Print","1x1 Thick Pole Print","1F 1x1F Vertical Corner Print","1x1F Vertical Edge Print","2x2x9 Barrel Print","2x2F L-Shaped Print","4-Sided Print 1x1","Bottom Print 2x2F","1x1f Thirds 1 Print","1F 1x2F Half-Plate Print","4h Panel 4x Print Inside","1F 2x2F Vertical Print"] {
            assert!(!reg(n), "should skip {n}");
        }
    }
}
