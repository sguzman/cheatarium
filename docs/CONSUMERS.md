# Cheatarium consumers

Cheatarium publishes **read-only**, revision-pinned, compressed JSON and a Rust client in `crates/cheatarium-client`. It is not an emulator and does not modify emulator repositories.

## SNES files

- `generated/v1/catalog.json`: the machine-readable console catalog.
- `generated/v1/games/snes.json.gz`: advisory title groups linking to their source records.
- `generated/v1/snes.json.gz`: full parsed source occurrences, original code strings, hints and provenance.
- `generated/v1/distribution.json`: SHA-256 hashes and byte sizes of all artifacts.

An emulator can download only these four files at a **pinned Cheatarium commit**; it does not need the raw archives. Filenames and title-group keys are suggestions, never trusted release/ROM identities.

## Rust client

The client exposes `load_catalog`, `load_game_candidates`, `load_platform`, `verify_platform_distribution`, and candidate title searches. Its CLI supports these local-only operations:

```sh
cargo run --release -p cheatarium-client --bin cheatarium-query -- games --db generated/v1 --platform snes --title 'Chrono Trigger' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- search --db generated/v1 --platform snes --title 'Chrono Trigger' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- verify --db generated/v1 --platform snes --json
```

`verify` confirms local files match the distribution manifest, but does not authenticate the manifest itself. Pin a trusted upstream Git commit or future immutable release.

## No implicit execution

Entries of `role: "code"` preserve native device-code strings; `role: "memory-entry"` preserves address/value metadata; `role: "section-heading"` marks a non-executable label. Codes are imported **unverified**. `source_enabled` means only that the original file marked them enabled.

Complete SNES Game Genie and Pro Action Replay source-code groups may include a derived `snes_decode` object when the source filename identifies the device; source files without a device label may also expose explicitly flagged syntax-only interpretations. The optional `snes_decode` object with ordered `{address_hex, value_hex}` entries, a CPU-bus address-space label, `interpretation_basis` (`declared-file-format` or `code-syntax`), and `compatibility: unverified-cartridge-build`. This is decoding, not execution permission. Undecodable placeholders and unknown formats have no `snes_decode` value. See [SNES codec guide](SNES-CODES.md).

Consumers must decide whether a code is compatible with the exact cartridge build and know how its specific Game Genie, Action Replay, or memory format behaves before applying anything. Nothing in this library activates a cheat automatically.

See [the v1 contract](INDEX-V1.md), [SNES decoding and coverage](SNES-CODES.md), and [distribution specification](DISTRIBUTION.md). The generated [SNES coverage report](../generated/v1/reports/snes-codec-coverage.json) is available for any consumer to inspect.
