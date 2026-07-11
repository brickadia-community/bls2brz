# bls → brz/brdb conversion quirks

Gotchas specific to mapping Blockland saves onto the new Brickadia format. Implementation lives
in `src/lib.rs` and `src/mappings.rs`.

## Color / material

- **sRGB → linear**: bls palette floats are gamma-space; gamma-expand before scaling to `0..255`
  (`gamma_expansion` in `lib.rs`), otherwise colors come out too dark/bright.
- **No palette on output**: brdb has no per-save color table, so the 64-color bls palette is
  resolved to concrete RGB per brick at convert time.
- **Alpha is not opacity on the brick color.** bls alpha (`< 255`) is turned into a *material*:
  - `a == 255` → material from `color_fx` (`3`=glow, `1|2`=metallic, else plastic), intensity `5`.
  - `a < 255` → `BMC_TranslucentPlastic`, intensity = alpha rescaled to `0..=10`
    (`alpha_to_intensity`).
- **`material_intensity` is `0..=10`, not `0..=255`.** Passing raw alpha (e.g. 81) makes the game
  reject the save with *"invalid material alpha, max is 10"*. This bit us; hence `alpha_to_intensity`.
- Opaque bricks use intensity `5` (brdb's neutral default) rather than `0`, because glow
  brightness rides on `material_intensity` — `0` would produce dead glow bricks.
- Open question: exact perceptual mapping of alpha → translucent-plastic intensity is a guess;
  eyeball a glass build in-game and tune the curve.

## Geometry

- **X/Y swap**: bls `position.1 → x`, `position.0 → y` (Blockland axes differ from Brickadia).
- **Scale**: bls studs `* 20` = Brickadia units. Unchanged from the old `brs` pipeline.
- **Size units**: a `BrickDesc.size` (u32 triple) casts to `BrickSize` (u16). Only applied to
  Procedural (`PB_`) assets; Basic (`B_`) assets ignore size.
- The heavy rotation logic (microwedge, ramps, ModTer inversion, `rotate_by_direction`) is copied
  verbatim from bls2brs and operates on a `u8` rotation `0..=3` + `Direction`, converted to the
  `Rotation` enum only at the final `Brick` construction.

## Brick mapping

- `src/mappings.rs` (≈1900 lines) maps bls `ui_name` → one-or-more Brickadia assets, either by
  literal table (`BRICK_MAP_LITERAL`) or regex (`BRICK_MAP_REGEX`, handles `NxM`, ramps, etc.).
  This layer is **format-agnostic** — it produces asset *names*, so it did not change in the brz port.
- One bls brick can expand to several output bricks (a `BrickMapping = Vec<BrickDesc>`), e.g. doors.
- Unmapped `ui_name`s are counted and reported as "unknown bricks" (usually add-on/mod bricks).

## Ownership / metadata

- A single PUBLIC owner is registered (`Guid::default()` / `Owner::default()`), every brick gets
  `owner_index = Some(0)`.
- Description is copied to `World.meta.bundle.description`, prefixed with a "Converted from … with
  bls2brz" line in `main.rs`.

## Things intentionally dropped

- bls "extra" lines (real owner IDs, events, named-brick config) — `bls` parses them, but the converter ignores them.
- `shape_fx` (undulo/water/etc.) — no mapping.
- Prints/decals — `print` field currently ignored.

## Verifying output

`examples/verify.rs` reads a produced `.brz` back and dumps a chunk's brick count, colors, and the
`material_intensity` range. Because of the brdb 0.5 reader enumeration bug (see
[brz-brdb-format.md](brz-brdb-format.md)), pass an explicit chunk index it printed in the fs tree:

```
cargo run --example verify -- MySave.brz <cx> <cy> <cz>
```

Final ground truth is always loading the save in Brickadia.
