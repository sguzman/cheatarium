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

An indexed code entry has an ordinal, description, optional code string, `source_enabled`, `verification`, `role`, and ordered `native_fields`. SNES entries may also have a strictly parsed optional `snes_decode` (format, source evidence basis, CPU bus label, unverified compatibility, and ordered address/value writes). Eight-hex-digit entries without declared device provenance use `raw-snes-address-value` rather than pretending to be Action Replay. Syntax-only decoding never implies ROM compatibility. Roles distinguish `code`, `memory-entry` and `section-heading`. Provenance retains source ID, repository, pinned revision, license, original path, archive path and Git blob SHA.

**Game-candidate bundles:** `generated/v1/games/<platform>.json.gz` contains advisory title groups with `key`, title hints, `source_record_ids`, distinct source IDs, region/format hints, counts, and `possible_title_collision`. An ungroupable record remains isolated with an `unresolved:` key.

**Reviewed gameplay-effect evidence:** `reviews/v1/claims.json` is a separately maintained register, published as `generated/v1/reviews.json` only after strict source/reference and observation-context validation. Future claims must cite an exact original source blob and cheat ordinal. Reports are never mislabeled as tested; observed and not-reproduced outcomes require a specific ROM SHA-256, emulator/core, version and date. The first five attributed external reports cover specific Donkey Kong and Donkey Kong Country codes, but no empirical test observations exist yet. These records remain evidence, not permission to activate cheats. See [effect review guide](EFFECT-REVIEWS.md).

**Lexical effect-tag bundles:** `generated/v1/tags/<platform>.json.gz` contains word-boundary matches between original cheat descriptions and the separately versioned, checked `generated/v1/taxonomy/effects-v1.json`. Every occurrence links back to the original source record and code ordinal and records the exact matched phrase. Labels are **English-language text cues only** and do not verify gameplay effects, game identity or cartridge compatibility. The catalog adds `tag_index_artifact` and `tag_matches`.

**Raw-code repetition bundles:** `generated/v1/repeats/<platform>.json.gz` lists exact original source-code strings appearing in distinct source files within the same filename-derived game key, region hint, explicit revision marker and declared device format. Each group links back to all original source-record IDs and ordinals, preserving distinct descriptions. It deliberately excludes native memory entries and does **not** assert identical effects or cartridge compatibility. The additive catalog fields `repeat_index_artifact` and `repeat_groups` describe these bundles.

**Catalog:** `generated/v1/catalog.json` references the source and game bundles for each console, with file and entry counts. The additive `decoded_snes_code_fields` reports the number of SNES source code fields decoded in full. `game_index_artifact` and `game_candidate_groups` are additive v1 fields.

**Source-code join semantics:** `interpretations/v1/snes.json` gives separately sourced partitions for documented SNES `+`-joined alternatives; it is copied to `generated/v1/interpretations/snes.json` and validated against exact original source blobs and ordinals. The original `code` is untouched. An unreviewed `+` join gets `composition.relation: "unresolved"`; an evidenced `revision-alternatives` group has explicit alternative component arrays and does **not** emit a combined `snes_decode` object. Neither means a ROM revision has been matched. See [SNES code decoding](SNES-CODES.md).

**ROM hash evidence:** `generated/v1/identities/snes.json` is a separately sourced (currently empty) registry of exact-file SHA-256 release claims, independent of filenames or cheat codes. It is referenced by the optional `identity_artifact` catalog field. A hash match never verifies a cheat.

**Distribution:** `generated/v1/distribution.json` lists SHA-256 and byte length for every artifact, including the ROM evidence registry, created and checked by `tools/build_distribution.py`.

## Interpretation rules

Source occurrences and filename-derived game groups do not prove game identity, ROM revision, device compatibility, or code function. Group keys may collide; different games must not be merged as verified merely because their slugs match. Different code formats must not be treated as raw memory writes without console-specific decoding.

Imported entries are unverified unless actually tested, and `source_enabled` is original metadata, **never** a request to activate a cheat.

Existing v1 source bundles remain backward compatible. New fields may be added; breaking semantic changes require a new major schema. Data revisions can change underlying records even when the contract version is unchanged. Consumers should pin one Git revision and verify its artifacts.

See [consumer guide](CONSUMERS.md) and [distribution/integrity contract](DISTRIBUTION.md).
