# Format notes

Working notes on the file formats bls2brz bridges, plus the quirks hit while writing the
converter. Sourced from the `bls` and `brdb` (0.4.0 local / 0.5.0 published)
crates and observed behaviour of Brickadia itself.

- [bls-format.md](bls-format.md) — Blockland `.bls` save format (input)
- [brz-brdb-format.md](brz-brdb-format.md) — Brickadia `.brz` / `.brdb` save format (output)
- [conversion-quirks.md](conversion-quirks.md) — gotchas mapping one to the other

These are field notes, not a spec. If something here disagrees with the game, trust the game
and fix the note.
