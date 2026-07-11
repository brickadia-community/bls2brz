# bls2brz

Convert Blockland saves (bls) to [Brickadia] saves in the new `brz`/`brdb` format.

Forked from [bls2brs], which targets the legacy `brs` format. bls2brz keeps the same
Blockland parsing and brick-name mappings and swaps the output layer to the [brdb] crate.

## Usage

Drag `.bls` files onto the executable (`bls2brz.exe` or `bls2brz`) to create corresponding
`.brz` files next to them.

Output format is chosen by extension: `.brz` (compressed archive, default) or `.brdb`
(SQLite directory format).

Not all Blockland bricks are supported, but the converter tries its best to support many variants.

## Notes

- Blockland color alpha is interpreted as material: opaque colors keep the material implied
  by the Blockland color effect (plastic/glow/metallic); translucent colors become
  translucent plastic, with the alpha stored as material intensity.

## Format notes

See [`docs/`](docs/) for field notes on the `.bls`, `.brz`, and `.brdb` formats and the quirks
of converting between them.

## Contributing

Pull requests are appreciated. If you encounter missing bricks, update `src/mappings.rs`.

[Brickadia]: https://brickadia.com
[bls2brs]: https://github.com/brickadia/bls2brs
[brdb]: https://github.com/brickadia-community/brdb
