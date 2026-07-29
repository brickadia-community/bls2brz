//! Generate a Blockland save containing the regular and optimized cube variants
//! supported by the converter.
//!
//! Usage:
//!   cargo run --example cube_variants_test -- <output.bls>

use bls2brz::bls::{Brick, Color, Save};

/// `(ui_name, grounded_center_z_in_blockland_units)`.
const CUBES: &[(&str, f32)] = &[
    // Regular baselines.
    ("2x Cube", 0.5),
    ("4x Cube", 1.0),
    ("8x Cube", 2.0),
    ("16x Cube", 4.0),
    ("32x Cube", 8.0),
    // Cubescape Optimizer variants, measured from their .blb headers.
    ("4x Cube 1h", 1.0),
    ("4x Cube 2h", 1.0),
    ("4x Cube 3h", 1.0),
    ("4x Cube 4h", 1.0),
    ("4x Cube 1v", 2.0),
    ("4x Cube 2v", 3.0),
    ("4x Cube 3v", 4.0),
    ("4x Cube 4v", 2.0),
    ("8x Cube 1h", 2.0),
    ("8x Cube 2h", 2.0),
    ("8x Cube 3h", 2.0),
    ("8x Cube 4h", 2.0),
    ("8x Cube 1v", 4.0),
    ("8x Cube 2v", 6.0),
    ("8x Cube 3v", 8.0),
    ("8x Cube 4v", 4.0),
    ("16x Cube 1h", 4.0),
    ("16x Cube 2h", 4.0),
    ("16x Cube 3h", 4.0),
    ("16x Cube 4h", 4.0),
    ("16x Cube 1v", 8.0),
    ("16x Cube 2v", 12.0),
    ("16x Cube 3v", 16.0),
    ("16x Cube 4v", 8.0),
    ("32x Cube 1h", 8.0),
    ("32x Cube 4h", 8.0),
    ("32x Cube 1v", 16.0),
    ("32x Cube 4v", 16.0),
    // Optimized cube variants from other packs.
    ("2x Cube 4x V", 2.0),
    ("2x Cube 4x H", 0.5),
    ("4x Cube 4x H", 1.0),
    ("4x Cube 2x V", 2.0),
    ("4x Cube 4x V", 4.0),
    ("4x Cube 4x4 V", 2.0),
    ("2x Cube 4x4 V", 1.0),
    ("4x8 Cube 1/2h", 0.5),
    ("4x16 Cube 1/2h", 0.5),
    ("32x Cube 1/16h", 0.5),
    ("4x Cube - No Overlap", 1.0),
    ("16x Cube 1/8h", 0.5),
    ("16x Cube Fixed", 4.0),
    (" 4x Cube Fix", 1.0),
    ("4x Cube Wall", 1.0),
    ("8x Cube Wall", 2.0),
    ("2x Cube Print", 0.5),
    ("4x Cube Print", 1.0),
    ("6x Cube Print", 1.5),
    ("8x Cube Print", 2.0),
];

const COLUMNS: usize = 6;
const CELL_SPACING: f32 = 40.0;

fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: cube_variants_test <output.bls>");
    let mut save = Save::new();

    for (index, (name, center_z)) in CUBES.iter().enumerate() {
        save.colors[index] = distinct_color(index);

        let column = (index % COLUMNS) as f32;
        let row = (index / COLUMNS) as f32;
        let mut brick = Brick::new(*name);
        brick.position = (column * CELL_SPACING, row * CELL_SPACING, *center_z);
        brick.color_index = index as u8;
        brick.raycasting = true;
        brick.collision = true;
        brick.rendering = true;
        brick.extras_mut().name_tag = Some(format!("{index:02}: {name:?}"));
        save.bricks.push(brick);
    }

    save.to_path(&output)
        .expect("failed to write cube test save");
    eprintln!("wrote {} cube variants to {output}", CUBES.len());
}

/// A high-contrast golden-angle palette with a unique hue for every variant.
fn distinct_color(index: usize) -> Color {
    let hue = (index as f32 * 0.618_034) % 1.0;
    let h = hue * 6.0;
    let chroma = 0.78;
    let x = chroma * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let value_offset = 0.18;
    Color {
        r: r + value_offset,
        g: g + value_offset,
        b: b + value_offset,
        a: 1.0,
    }
}
