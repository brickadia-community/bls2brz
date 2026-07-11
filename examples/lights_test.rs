//! Generate a `.bls` containing one brick per light datablock, laid out in a
//! grid, so the same save can be loaded in Blockland and converted to Brickadia
//! for a side-by-side light comparison.
//!
//! Usage: `cargo run --example lights_test -- <output.bls>`

use bls2brz::bls::{Brick, Color, Named, Save};

/// Every light datablock we want to eyeball, in grid order. Includes the special
/// ones we deliberately skip (RGB/Alarm/Strobe) so their absence is visible.
const LIGHTS: &[&str] = &[
    // focused colors
    "Player's Light",
    "Orange Light",
    "Yellow Light",
    "Red Light",
    "Blue Light",
    "Cyan Light",
    "Green Light",
    "Purple Light",
    // wide ambients
    "White Amb.",
    "Orange Amb.",
    "Yellow Amb.",
    "Red Amb.",
    "Blue Amb.",
    "Cyan Amb.",
    "Green Amb.",
    "Purple Amb.",
    // named / bespoke
    "Candle Light",
    "Candlelight Warm",
    "Bright",
    "Bulb Light",
    "Orange Warm",
    "Red Warm",
    "Cyan Warm",
    // specials we intentionally drop
    "RGB",
    "Alarm",
    "Strobe",
];

const COLS: usize = 6;
const SPACING: f32 = 4.0;

fn main() {
    let out = std::env::args().nth(1).expect("usage: lights_test <output.bls>");

    let mut save = Save::new();
    // A light grey so the bricks read against the void.
    save.colors[0] = Color { r: 0.8, g: 0.8, b: 0.8, a: 1.0 };

    for (i, name) in LIGHTS.iter().enumerate() {
        let col = (i % COLS) as f32;
        let row = (i / COLS) as f32;

        let mut brick = Brick::new("1x1");
        brick.position = (col * SPACING, row * SPACING, 0.5);
        brick.color_index = 0;
        brick.raycasting = true;
        brick.collision = true;
        brick.rendering = true;

        let extras = brick.extras_mut();
        // A floating nametag so each light is identifiable in-game.
        extras.name_tag = Some((*name).to_string());
        extras.lights.push(Named::new(*name));

        save.bricks.push(brick);
    }

    save.to_path(&out).expect("failed to write bls");
    eprintln!("wrote {} lights to {}", LIGHTS.len(), out);
}
