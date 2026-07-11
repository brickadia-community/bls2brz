# Blockland `.bls` save format

Input format. Notes derived from the `bls` crate (`brick.rs`, `read.rs`).

## Overall structure

Plain **text**, **CP1252** encoded (not UTF-8 — matters for description/print strings with
accented chars). Line-oriented. Layout:

1. Magic comment line: `This is a Blockland save file.  You probably shouldn't modify it...`
2. Description: a line with the line-count `N`, then `N` lines of description (escaped).
3. **Colorset**: exactly **64** lines, each `r g b a` as floats in `0.0..=1.0`.
4. Optional brick-count line (Blockland writes it, but it is **not guaranteed** present or
   correct — treat as a hint only).
5. Brick lines, each optionally followed by "extra" lines (owner, events, named-brick data).

## Colorset (palette)

- Fixed size **64** entries, indexed `0..=63` by each brick's `color_index`.
- Values are floats `0..1` in **sRGB / gamma space**. To match Brickadia's colors they need
  gamma expansion (sRGB → linear) before scaling to `0..255`. See `map_color` in `src/lib.rs`.
- Alpha is present in the palette but most palette entries are opaque (`a = 1.0`). Translucent
  bricks (glass) carry `a < 1.0` here or via a color override.

## Brick line fields

Order (from `read.rs`):

| Field | Type | Notes |
|---|---|---|
| `ui_name` | quoted string | The `uiName` of the `fxDTSBrickData` datablock, e.g. `"2x4"`, `"1x1 Round"`, `"Music Brick"`. This is the join key to Brickadia assets — see `src/mappings.rs`. |
| `position` | `(f32, f32, f32)` | In **studs**. Note Blockland's X/Y are swapped relative to Brickadia (the converter maps `pos.1→x`, `pos.0→y`). |
| `angle` | `u8` | Rotation, `0..=3` (quarter turns). |
| `is_baseplate` | bool | Datablock-is-a-baseplate flag. |
| `color_index` | `u8` | Index into the 64-color palette, `0..=63`. |
| `print` | string | Print/decal name for print bricks; `""` = none. |
| `color_fx` | `u8` | Color effect. Used for material: `3` = glow, `1`/`2` = metallic-ish (chrome/etc.), else plastic. |
| `shape_fx` | `u8` | Shape effect (undulo, water, …). Not currently mapped. |
| `raycasting` | bool | Can be raycast against. |
| `collision` | bool | Objects collide with it. |
| `rendering` | bool | Visible. |

## Quirks / gotchas

- **X/Y swap** vs Brickadia — easy to get mirrored builds if ignored.
- **Stud → Brickadia unit scale**: bls2brs/bls2brz multiply position by **20**.
- `bls` **does** parse "extra" `+-` lines (owner, events, items, lights, emitters, vehicles,
  named-brick tags) onto `Brick`, and keeps unrecognized ones verbatim in `BrickExtras.unknown`.
  The converter currently uses only the base fields and drops all of it.
- `bls` reads the whole save into `Save { description, colors: [Color; 64], bricks }` up front,
  so the converter walks `save.bricks` rather than streaming.
- Datablock names are mod-dependent: a `.bls` referencing add-on bricks not installed still
  lists their `name` (`uiName`), but there may be no Brickadia equivalent (shows up as "unknown brick").
- `color_fx` / `shape_fx` integer meanings are only partially documented; `bls` stores them as
  raw `u8` without validation.
