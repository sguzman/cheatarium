# Repeated source-code text

Cheatarium exposes a separate, read-only index of **identical original encoded cheat strings** found in multiple source files. This is a candidate discovery and curation aid, not a declaration that those records have the same gameplay effect.

## Motivation

Two archived files may contain the same code text, use different descriptions, or describe distinct situations. Automatically overwriting one or combining their metadata would destroy evidence. Likewise, a visually similar code on a different console, game build, region, or cheat device is not automatically interchangeable.

## Conservative matching

A repetition group requires **at least two distinct source-record IDs**, both containing the exact same nonempty original code string. Groups are partitioned by:

- platform,
- filename-derived candidate game key (not a verified game ID),
- region hint (including a distinct `null` for missing region),
- explicit revision or edition marker from the source filename (including a distinct `null` for unknown),
- source-declared device format (including a distinct `null` for unlabeled codes),
- raw code string, **byte-for-byte as parsed** (case, whitespace and compound ordering are not normalized).

Only entries with role `code` participate. Native `memory-entry` fields and headings are intentionally excluded from these encoded-code comparisons. The original full file, all entries, descriptions, original ordinals and attribution remain unchanged in the source index.

The `relation` field is always `identical-raw-code-text-within-advisory-filename-bucket`. Both `confirmed_equivalent_cheat` and `verified_rom_compatibility` are always false. Even an exact string match **cannot prove that the cartridge edition or effect is the same**, especially when source filenames are incorrect.

## Offline consumer

```fish
cargo run --release -p cheatarium-client --bin cheatarium-query -- repeats --db generated/v1 --platform snes --game-key super-mario-world --json
```

Each group reports `description_variants` and `description_text_varies`, computed by literal comparison of nonempty original descriptions. This is a prompt for editorial review, not a semantic contradiction. Use `--varying-descriptions` with the `repeats` command to limit output to groups with different source wording.

Each group returns its original string and an occurrence list of `{source_record_id, ordinal, description}`. Resolve `source_record_id` in `generated/v1/snes.json.gz` to recover the full source provenance. The query is non-executing and does not access ROMs.

All generated `repeats/<platform>.json.gz` bundles are checksum-verified by `distribution.json`; `tools/build_distribution.py` validates source references and explicit bucket boundaries. Code text is not deleted or consolidated in either the archive or curated records.

## Next step

Actual cheat equivalence requires linking to independently evidenced game/revision identities, device-specific semantics, and reviewed descriptions or empirical testing. Until then the results remain repeat candidates.
