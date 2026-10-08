# Cheatarium index v1

The indexer is a **repeatable Rust build**, not a manual snapshot. Data exports are emulator-independent and never modify original sources.

## Build

```sh
cargo test --workspace
cargo run --locked --release -p cheatarium-index -- --root . --out generated/v1
python3 tools/build_distribution.py --write
python3 tools/build_distribution.py --check
```

Use `--systems snes,nes` for bounded development builds, not for refreshing the full distributed catalog.

## Layers

**Source archive:** unchanged `.cht` files under `archive/libretro/cht/`; per-file upstream path, revision, and original Git blob SHA in `sources/libretro-database.json`.

**Source bundles:** `generated/v1/<platform>.json.gz` contains `schema_version: 1`, `platform`, compatibility/identity rules and original source occurrences. Each `record` exposes a stable source occurrence `id`, advisory `title_hint` and `candidate_game_key`, original filename, optional region/format hints, `codes`, and `provenance`.

An indexed code entry has an ordinal, description, optional code string, `source_enabled`, `verification`, `role`, and ordered `native_fields`. SNES entries may also have a strictly decoded optional `snes_decode` (format, CPU bus label, unverified compatibility, and ordered address/value writes). Roles distinguish `code`, `memory-entry` and `section-heading`. Provenance retains source ID, repository, pinned revision, license, original path, archive path and Git blob SHA.

**Game-candidate bundles:** `generated/v1/games/<platform>.json.gz` contains advisory title groups with `key`, title hints, `source_record_ids`, distinct source IDs, region/format hints, counts, and `possible_title_collision`. An ungroupable record remains isolated with an `unresolved:` key.

**Catalog:** `generated/v1/catalog.json` references the source and game bundles for each console, with file and entry counts. The additive `decoded_snes_code_fields` reports the number of SNES source code fields decoded in full. `game_index_artifact` and `game_candidate_groups` are additive v1 fields.

**Distribution:** `generated/v1/distribution.json` lists SHA-256 and byte length for every artifact, created and checked by `tools/build_distribution.py`.

## Interpretation rules

Source occurrences and filename-derived game groups do not prove game identity, ROM revision, device compatibility, or code function. Group keys may collide; different games must not be merged as verified merely because their slugs match. Different code formats must not be treated as raw memory writes without console-specific decoding.

Imported entries are unverified unless actually tested, and `source_enabled` is original metadata, **never** a request to activate a cheat.

Existing v1 source bundles remain backward compatible. New fields may be added; breaking semantic changes require a new major schema. Data revisions can change underlying records even when the contract version is unchanged. Consumers should pin one Git revision and verify its artifacts.

See [consumer guide](CONSUMERS.md) and [distribution/integrity contract](DISTRIBUTION.md).
