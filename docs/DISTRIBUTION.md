# Cheatarium v1 consumer distribution

This document covers Cheatarium's exported artifacts only. No emulator is modified by building or consuming them.

## Distribution artifacts

Every successful index build produces the following under `generated/v1/`:

- `catalog.json`: versioned catalog of consoles, per-platform source bundles, advisory game groups and additive evidence indexes.
- `<platform>.json.gz`: complete, unaltered-in-meaning *parsed source occurrences*, including code entries and byte-level upstream provenance.
- `games/<platform>.json.gz`: small, **advisory filename-derived game groups** with links back to source occurrence IDs, regions, formats, and counts.
- `reports/snes-codec-coverage.json`: deterministic SNES device/syntax interpretation coverage audit, including rejected/ambiguous codes and source samples.
- `reviews.json`: independently evidenced effect-review register, currently with five external reports and zero tested observations; separately validated and included in checksums.
- `taxonomy/effects-v1.json`: versioned lexical effect taxonomy (included once in the manifest).
- `tags/<platform>.json.gz`: per-console lexical description phrase hits, each linked back to its source occurrence; recomputed from originals during validation.
- `repeats/<platform>.json.gz`: per-console exact-text repetition candidates with source references, included in the manifest for all 36 bundles.
- `reports/snes-composition-review-queue.json`: deterministic summary of unresolved SNES `+` source-code joins, ranked for research and linked to exact original source records; not compatibility evidence.
- `interpretations/snes.json`: separately sourced, independently validated SNES source-code group relationships. Revision alternatives preserve version-specific partitions without inventing ROM identity or runnable combined writes.
- `interpretations/snes-published-groups.json`: externally witnessed historical multi-part listings, pinned to exact original source ordinals and upstream Git blobs; these do **not** resolve execution semantics or authorize applying codes.
- `identities/snes.json`: independent SHA-256 whole-file ROM release evidence registry, copied from `identities/v1/snes.json` and checksummed alongside the cheat bundles. This is **not** a list of verified cheat-to-ROM mappings.
- `distribution.json`: deterministic SHA-256 digests and byte lengths for every exported artifact (including the coverage report), aggregate counts, SNES historical-publication witness coverage, and additive `decoded_snes_code_fields`. A distribution manifest does **not** sign itself.

The source `.cht` files remain in `archive/` and are the authority for original bytes. No ROM images, firmware, or save data are required or supplied.

## Candidate groups are not identified games

`games/snes.json.gz` groups occurrences by a conservative title slug. It is for **browsing and narrowing a search**, not asserting a common ROM revision or equivalent cheats. Each group contains `key`, `title_hint`, `alternate_title_hints`, `source_record_ids`, `source_ids`, `region_hints`, `format_hints`, and code counts. `identity_confidence` remains `filename_candidate_only` and `possible_title_collision` flags differing title hints that map to the same key.

Any record whose title cannot yield a usable key stays in its own `unresolved:<source-occurrence-id>` group. Do not auto-apply codes based on a group, region hint, format hint, title text, or source-enabled flag.

## Rust consumer

The `cheatarium-client` crate offers `load_catalog`, `load_platform`, `load_game_candidates`, `verify_platform_distribution`, source-record search, and candidate-game search. All operate on local files and never activate cheats.

Example usage:

```sh
cargo run --release -p cheatarium-client --bin cheatarium-query -- games --db generated/v1 --platform snes --title "Chrono Trigger" --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- search --db generated/v1 --platform snes --title "Chrono Trigger" --json
```

## Integrity and repeatability

`python3 tools/build_distribution.py --check` verifies SHA-256 hashes/byte lengths, parses both exports for every platform, and ensures every source record belongs to exactly one candidate group. `--write` regenerates the manifest after a Rust build; it is deterministic and network-free.

Consumers should pin one Cheatarium commit or future immutable release and validate `distribution.json` before trusting downloaded artifacts. `cheatarium-query verify --db generated/v1 --platform snes --json` checks the local SNES files against the manifest; it does not authenticate the manifest itself. SHA-256 confirms files match the pinned manifest; it does **not** prove that cheats work, that the manifest was authenticated, or that the game title matches a user's ROM.

Schema `v1` remains backward compatible by **adding** the `game_index_artifact` and `game_candidate_groups` catalog fields. Existing `<platform>.json.gz` source bundles and the `search` CLI continue to work. New clients may discover `games/<platform>.json.gz` via the new optional fields.

## Boundaries

Cheatarium will later add supported code-device decoders, verified region/ROM/build compatibility, and cross-source equivalence annotations. Those future features must be evidence-based. The independent emulator consuming these files decides if, when, and how it executes any cheat.
