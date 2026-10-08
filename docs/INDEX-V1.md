# Cheatarium index v1

The indexer is a **repeatable Rust build**, not a hand-maintained snapshot.

## Build

\`\`\`sh
cargo test --workspace
cargo run --release -p cheatarium-index -- --root . --out generated/v1
# Optional: --systems snes,nes
\`\`\`

This writes a stable \`catalog.json\` plus **one compressed \`<platform>.json.gz\` per imported console**. Both are checked into \`generated/v1/\` by CI, so clients may consume a pinned Git revision or a future GitHub release without cloning source archives.

The indexer currently parses Libretro \`.cht\` files only. Additional source formats will become separate adapters; every record already carries source ID, repository URL, exact snapshot commit, upstream path, archive path and original Git blob SHA.

### Per-console payload

\`\`\`json
{
  "schema_version": 1,
  "platform": "snes",
  "game_identity_rule": "filename-derived suggestion, not verified ROM identity",
  "compatibility_rule": "never auto-apply a code without confirmed release/build compatibility",
  "records": [
    {
      "id": "libretro-database:cht/.../Donkey Kong Country (USA) (Game Genie).cht",
      "title_hint": "Donkey Kong Country",
      "candidate_game_key": "donkey-kong-country",
      "identity_confidence": "filename_heuristic_only",
      "raw_filename": "Donkey Kong Country (USA) (Game Genie).cht",
      "region_hint": "USA",
      "format_hint": "game-genie",
      "declared_cheats": 51,
      "parse_warnings": [],
      "codes": [{
        "ordinal": 0,
        "description": "Invincible",
        "code": "1DCC-CA7A",
        "source_enabled": false,
        "verification": "unverified"
      }],
      "provenance": {
        "source_id": "libretro-database",
        "repository": "https://github.com/libretro/libretro-database",
        "revision": "...",
        "license": "CC-BY-SA-4.0",
        "upstream_path": "...",
        "archive_path": "...",
        "git_blob_sha": "..."
      }
    }
  ]
}
\`\`\`

Fields in this example are illustrative and abbreviated. The real outputs contain **all** indexed \`.cht\` source records and their parseable code entries. The raw archives remain authoritative for bytes and any unsupported fields. Metadata anomalies become parse warnings; the importer and Python validator retain separate byte-integrity checks.

## Consumer contract

- \`catalog.json\` selects available platform bundles; artifact filenames are relative to its directory.
- Gzip output has a zero modification timestamp, stable ordering, and no build-time timestamps, so the same repository revision yields byte-identical output with the same build dependencies.
- A record is a **source occurrence**, not a unique game or an endorsement of a code. \`candidate_game_key\` is advisory and may collide; \`raw_filename\` and exact provenance are never lost.
- A code can require a specific cartridge revision, emulator device, core or memory domain. The filename-derived region/format hints are **not** enough to enable a code safely.
- \`source_enabled\` is how the upstream file represented its own default, **not permission to enable the code in a consumer**. No cheats are automatically activated.
- Do not interpret the original cheat-code strings as native memory-write instructions. Device formats need verified console-specific decoding/execution.
- Consumers should check \`schema_version\`, accept unknown future fields, and fail safely on unsupported versions.

This is the first transport contract for **Starbyte and future emulators**. A stable game/build resolver, format-specific execution adapters, and a shared Rust consumer library are separate upcoming tasks.

## Mixed provenance

Each source file is retained independently. Distinct upstream files containing identically named codes remain distinct \`records\`; automatic effect-level merging is deliberately deferred until we can use reliable game/build identity and evidence-based equivalence, not just string similarity.
