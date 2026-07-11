# Brickadia `.brz` / `.brdb` save format

Output format. Notes from the `brdb` crate (0.4.0 local checkout at `/home/kevin/src/brdb`,
0.5.0 published on crates.io / `brickadia-community/brdb`) and observed game behaviour.

## Two containers, one model

Both wrap the **same in-memory `World`**; only the on-disk container differs:

- **`.brz`** — a single file: magic `BRZ\0`, header, index, then blobs, **zstd compressed**
  (default level 14; `save_uncompressed` exists). Internally a virtual filesystem tree.
- **`.brdb`** — a **SQLite 3** database (a directory-like blob store), uncompressed.

Choose by extension. `World::write_brz(path)` / `World::write_brdb(path)`. Round-trip converters
exist both ways (`brdb_to_brz`, `brz_to_brz` examples).

### Virtual filesystem tree (inside a `.brz`)

```
Meta/Bundle.json         name, version, authors, description, timestamps
Meta/World.json          environment name (default "Plate")
Meta/Screenshot.jpg      optional
Meta/Thumbnail.png       optional
World/0/                 the (single, currently) world
  Owners.mps + .schema
  GlobalData.mps + .schema     shared asset/material name tables
  Bricks/Grids/1/Chunks/<cx>_<cy>_<cz>.mps    main grid = grid 1
  Bricks/Grids/1/ChunkIndex.mps
  Bricks/*Shared.schema        ChunkIndexShared, ChunksShared, WiresShared, ComponentsShared
  Entities/...
```

`.mps` blobs are **msgpack** encoded against the accompanying `.schema`.

## The `World`

```
World {
  meta: WorldMeta { bundle, screenshot, thumbnail, world }
  owners: IndexMap<Guid, Owner>
  bricks: Vec<Brick>                    // main grid (grid id 1)
  grids: Vec<(Entity, Vec<Brick>)>      // dynamic grids, each anchored to an entity
  wires: Vec<WireConnection>
  entities: Vec<Entity>
}
```

- **Main grid is grid id 1.** Its bricks are stored unshifted. Bricks added to a dynamic grid
  via `add_brick_grid` are shifted by `-CHUNK_HALF (-1024)` to centre them in the entity's chunk.
- `World` **does not yet serialize minigame or environment** data (TODO in the crate).

## Brick

```
Brick {
  id: Option<usize>
  asset: BrickType                      // Basic(name) | Procedural { asset, size }
  owner_index: Option<usize>            // index into World.owners
  position: Position { x,y,z: i32 }
  rotation: Rotation                    // Deg0 | Deg90 | Deg180 | Deg270
  direction: Direction                  // X/Y/Z Positive/Negative (+ MAX sentinel)
  collision: Collision
  visible: bool
  color: Color { r,g,b: u8 }            // RGB only, no alpha
  material: BString                     // "BMC_*"
  material_intensity: u8                // 0..=10 !!
  components: Vec<Box<dyn BrdbComponent>>
}
```

### `BrickType` — Basic vs Procedural

- **`Procedural { asset, size: BrickSize }`** — resizable bricks, asset names start with `PB_`
  (e.g. `PB_DefaultBrick`, `PB_DefaultTile`, `PB_DefaultMicroBrick`). Size is `u16` studs (x,y,z).
- **`Basic(name)`** — fixed-shape named bricks, `B_...` (e.g. `B_1x1_Round`). **Carries no size**;
  the shape is baked into the asset. Passing a size here is meaningless.
- Converter rule: `asset.starts_with("PB_")` → Procedural, else Basic.

### Color & material — the big gotcha

- `Color` is **RGB only** (`u8` each). There is **no per-brick opacity** channel on the color.
- On disk each brick color is a `SavedBrickColor { r, g, b, a }` where **`a` = `material_intensity`**,
  NOT opacity. (See brdb's own `write_brz` example asserting `color.a == 5` for a default brick.)
- **`material_intensity` range is `0..=10`.** The game rejects out-of-range values with
  *"invalid material alpha, max is 10"*. brdb's default brick uses `5`. Do **not** shove a 0..255
  alpha in here (that was a real bug — 81 got rejected).
- Materials (`brdb::assets::materials::*`, `BString` consts): `BMC_Plastic`, `BMC_Glass`,
  `BMC_TranslucentPlastic`, `BMC_Glow`, `BMC_Metallic`, `BMC_Hologram`, `BMC_Ghost`.
- Translucency is expressed by **material choice** (`BMC_TranslucentPlastic`/`BMC_Glass`) +
  intensity, not by a color alpha.

### Orientation

- `rotation` (`Deg0..270`) and `direction` (6 axes) are separate.
- Packed on disk: `orientation_byte = (direction as u8) << 2 | (rotation as u8)`.
- `Direction` has an extra `MAX` sentinel variant — invalid, don't emit it.
- `Direction`/`Rotation` derive `Copy`/`Default` but **not `PartialEq`** — use `matches!` to
  compare, not `==`.

### Collision

```
Collision { player, weapon, interact, tool, physics: bool,   // all default true
            player1, player2, player3: Option<bool> }         // added in 0.5
```
Construct with `..Default::default()` to stay forward-compatible across versions.

## Coordinates & chunks

- Units: same Brickadia internal units as the old `brs` format (1 stud plate ≈ default brick
  `size (5,5,6)`; a plate is 2 units tall). Blockland-stud positions are `* 20`.
- **Chunks are 2048³ units**, `CHUNK_HALF = 1024`. A world position maps to
  `(ChunkIndex, RelativePosition)` where relative coords are `-1024..=1023`.
- Chunk files are named `<cx>_<cy>_<cz>.mps`. Negative world coords → negative chunk indices
  (e.g. a brick at `x=-4000` lands in chunk `-2`, not `-1`).

## Storage: structure-of-arrays (SoA)

Chunks store bricks column-wise, not as an array of structs:
`brick_sizes`, `brick_type_indices`, `owner_indices`, `relative_positions`, `orientations`,
per-flag collision bitfields (`collision_flags_player`, `_weapon`, `_interaction`, `_tool`,
`_physics`), `colors_and_alphas`, plus `procedural_brick_starting_index` /
`brick_size_counters` bookkeeping. Basic bricks sort before procedural ones.

## Owners

- `owners: IndexMap<Guid, Owner>`; `owner_index` is the insertion position.
- `Owner::default()` = PUBLIC, `Guid::default()` = all four `u32` = `u32::MAX`.
- `Guid { a,b,c,d: u32 }` ⇄ `uuid::Uuid`.

## Reader quirks (brdb 0.5)

- `reader.brick_chunk_index(1)` **enumeration throws** `Schema(ArrayIndexOutOfBounds{index:0,
  len:0})` on multi-chunk saves — reproduced even with brdb's own writer. Work around by reading
  a known chunk directly: `reader.brick_chunk_soa(1, (cx,cy,cz).into())`.
- `soa.iter_bricks(chunk.index, global_data)` also threw `ArrayIndexOutOfBounds` (needs the
  global asset table). For quick verification, read `relative_positions` / `colors_and_alphas`
  off the SoA directly instead. See `examples/verify.rs`.

## Version drift

- Local checkout is **0.4.0**; the published/upstream crate is **0.5.0**. Known break:
  `Collision` gained `player1/2/3` in 0.5. Re-check struct fields after any bump — the compiler
  catches most of it.
