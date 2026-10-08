# Roadmap and acquisition queues

**North star:** a huge, pleasant, searchable collection, where original cheat files and meaningful game/edition/device relationships are both preserved.

## Collection pipeline

1. **Discover** a source; identify platforms, file formats, size, provenance, and licenses.
2. **Clear** reuse rights or mark the source link-only.
3. **Acquire** a pinned snapshot (start with bounded batches to audit the pipeline).
4. **Inventory** imported files and validate integrity.
5. **Normalize** into per-game curated records without discarding raw material.
6. **Verify** codes against actual game builds and supported emulators.
7. **Publish** searchable indexes, then integrations/exporters.

## Platform queue

- [x] Register console/handheld platform IDs.
- [x] Start Libretro NES and SNES proof-of-concept imports.
- [x] Import 39 Libretro console/handheld source collections with pinned per-file provenance (23,382 files, 36 canonical console bundles).
- [x] Add Libretro N64, Nintendo DS, Virtual Boy and FDS.
- [ ] Add GameCube, Wii, 3DS, Wii U and Switch from other sources.
- [x] Add Libretro Master System, Game Gear, Sega CD, 32X, Saturn, Dreamcast and SG-1000.
- [ ] Expand Sony beyond the imported PS1/PSP collections: PS2, PS3, Vita, PS4.
- [ ] Evaluate Nintendo Switch title-ID/build-ID cheat datasets, PS2 PCSX2 `.pnach` collections, and GameCube/Wii Gecko codes.
- [x] Add Libretro Atari, NEC, SNK handheld, WonderSwan, ColecoVision, Intellivision and GX4000.
- [ ] Evaluate Xbox families, Neo Geo AES/CD, 3DO and other collections.
- [ ] Add link-only indexes for valuable sources whose redistribution rights cannot be confirmed.

## Infrastructure queue

- [x] Brief, purposeful README and design/ingestion documentation.
- [x] Source manifests with exact upstream paths/revisions/blob IDs.
- [x] Game/edition/format schema and integrity validator.
- [x] Automatic upstream **bulk importer** with platform selection, limits, and deterministic records.
- [x] Generate versioned compressed per-console source-record indexes in Rust (36 consoles).
- [x] Generate advisory game-title catalogs with source links for all 36 consoles.
- [x] Publish SHA-256 artifact manifests with Rust client integrity checks.
- [x] Add source-evidenced, exact-file ROM fingerprint lookup with ambiguity and size-conflict handling; publish an initially empty SNES identity registry to avoid invented mappings.
- [x] Add local-only, ZIP-streaming SNES ROM metadata inventory scanner; do not collect or upload ROMs.
- [ ] Acquire independently sourced and reviewed release hashes, then establish reliable cartridge/build identity and canonical cross-source game records.
- [x] Publish cross-source exact-raw-code repetition candidates while preserving every original occurrence, region hint, format label, revision marker and description.
- [ ] Establish semantic equivalence and deduplicate only after game/release identity and effect validation.
- [x] Add case-insensitive, provenance-preserving cheat-effect text search in the Rust client and CLI.
- [x] Add exact source-ID and provenance-declared device-format filters for effect search.
- [x] Publish English lexical effect-category signals with inspectable phrases, source occurrence links, query filters and cross-file validation.
- [x] Add independently referenced review registry, validated original-record/ordinal attribution and explicit report-versus-observation evidence with exact-ROM test contexts.
- [x] Ground the first five external `reported` claims in dated historic cheat documents, with specific original source file blobs and ordinals (SNES DKC and NES DK).
- [ ] Acquire empirical cheat tests tied to exact ROM/build SHA-256 and version-specific device/core semantics; no empirical observation claims exist yet.
- [x] Add source-bound, independently cited SNES revision-alternative partitions for 27 DKC `+` records, including five two-code-per-version partitions, preserve raw codes and prevent combined decoder output for these exact records.
- [x] Produce a deterministic source-linked priority queue for unresolved SNES `+` records, with exact raw samples and filename-candidate counts.
- [x] Add lossless, paginated, read-only SNES source dossiers for any unresolved filename candidate, preserving descriptions, ordinals, original code text and upstream provenance.
- [x] Record seventeen source-exact historical publication witnesses across six SNES games, with separately validated/checksummed provenance and no claims of successful execution.
- [x] Expose historical SNES publication witnesses through the offline Rust consumer and CLI, with exact original-ordinal validation and integrity-checked query output.
- [x] Cluster near-identical unresolved SNES source strings into source-local, potentially overlapping text families for research; never infer executable combinations or release compatibility.
- [ ] Review remaining unresolved `+` joins, distinguish authentic multi-code device programs from revision alternatives, and eventually map verified builds to alternatives.
- [ ] Expand multilingual phrase cues and contributor/author attribution.
- [ ] Reliable console-specific converters/exporters.
- [x] Add read-only Rust index client and searchable CLI.
- [x] Add exact original device-code text lookup with source ordinals, provenance and safe pagination; do not infer cheat equivalence from shared strings.
- [ ] Add a pleasant desktop/game cheat explorer.
- [x] Implement SNES Game Genie / Pro Action Replay decoding, plus provenance-conscious syntax-only interpretation for unlabeled codes (over 66,000 fields interpreted); publish an audited coverage report with source-join semantics tracked separately.
- [ ] Add console-specific codecs beyond SNES; emulator integration remains owned by its separate projects.

Keep the README about the project; keep work tracking here.
