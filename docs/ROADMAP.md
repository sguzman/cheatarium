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

## Acquisition status (2026-10-10)

**33,091 actual original cheat files archived**, including **340 newly acquired in this continuation**: 62 original PS2 PNACH and 278 code-bearing Dolphin GameCube/Wii INIs. **33,057 files are searchable across 44 console bundles**; 34 original Dolphin INIs have unresolved family labels and remain safely archived. The current catalog exposes 1,257,967 code-bearing source entries; it does not assert unique effects, game compatibility or successful execution. Active priority remains acquiring further independently reusable console cheats, not UI work.

## Platform queue

- [x] Register console/handheld platform IDs.
- [x] Start Libretro NES and SNES proof-of-concept imports.
- [x] Import 39 Libretro console/handheld source collections with pinned per-file provenance (23,382 files, 36 canonical console bundles).
- [x] Add Libretro N64, Nintendo DS, Virtual Boy and FDS.
- [x] Archive 644 Nintendo 3DS and 491 Nintendo Switch native cheat files from pinned Sharkive (1,135 new original source files; unverified).
- [x] Index 3DS title-ID and Switch title/build-ID native cheat sections without inventing game names, build compatibility or activation semantics (12,274 + 6,150 source code sections).
- [x] Preserve 21 GameCube/Wii VC AR/Gecko original INI files and 28 Wii/Wii U Gecko code documents from independently licensed upstream collections.
- [x] Acquire 278 code-bearing Dolphin GameSettings INIs with original project COPYING and license variants: 210 GameCube, 34 Wii and 34 with unresolved disc-system identity.
- [x] Acquire and index 62 independent AeroWidescreen PS2 PNACH files retaining native region/CRC keys, authors and MIT notices.
- [ ] Expand GameCube/Wii/Wii U and other missing console source collections beyond these additional authors, and acquire independent 3DS/Switch/Vita sources where original-code reuse rights permit.
- [x] Add Libretro Master System, Game Gear, Sega CD, 32X, Saturn, Dreamcast and SG-1000.
- [x] Archive 5,445 original GoldHEN PS4-oriented native JSON/MC4/SHN/XML files and 2,540 original Artemis PS3 NCL cheat files, retaining exact revisions, original authors and license notices.
- [x] Index original PS3 NCL, PS4 GoldHEN JSON/SHN/MC4/XML, and GameCube/Wii/Wii U Gecko/AR source records without fabricating decoded cheats or game compatibility.
- [x] Acquire and index 110 original PS2 PNACH cheat files from pinned CookiePLMonster, plus 67 PS1, 4 PSP, and 2 GameCube native cheat files with author attribution.
- [ ] Expand Sony coverage with additional independent PS2 and Vita collections; GoldHEN PS2-era title IDs are not native PS2 coverage.
- [ ] Review additional Nintendo Switch title/build-ID databases, PS2 PCSX2 `.pnach` collections and GameCube/Wii Gecko sources whose reuse rights remain unresolved.
- [x] Add Libretro Atari, NEC, SNK handheld, WonderSwan, ColecoVision, Intellivision and GX4000.
- [ ] Evaluate Xbox families, Neo Geo AES/CD, 3DO and other collections.
- [ ] Add link-only indexes for valuable sources whose redistribution rights cannot be confirmed.

## Infrastructure queue

- [x] Brief, purposeful README and design/ingestion documentation.
- [x] Source manifests with exact upstream paths/revisions/blob IDs.
- [x] Game/edition/format schema and integrity validator.
- [x] Automatic upstream **bulk importer** with platform selection, limits, and deterministic records.
- [x] Generate versioned compressed per-console source-record indexes in Rust (44 consoles).
- [x] Generate advisory game-title / source-ID catalogs with provenance links for all 44 consoles.
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
- [x] Add read-only, paginated original-source inspection so console consumers can browse all imported code, native memory, and section-heading entries without title guesses.
- [x] Resolve exact candidate-game keys to checksum-checked, paginated original-source summaries with strict source-link and count validation.
- [x] Expose paginated console catalogs in the offline CLI and a complete platform → candidate game → exact source → original entries browse path.
- [x] Browse a candidate game's original cheat entries across all linked archives with optional source/role filters and untouched duplicate ordinals.
- [x] Search imported cheat descriptions within one advisory game by case-insensitive text, while preserving original code strings, provenance and source roles.
- [x] Resolve a single original cheat by exact source record ID and original ordinal for stable, provenance-backed detail views independent of array offsets or repeated code text.
- [x] Add exact upstream-collection, region and declared-device-format facets for game-level cheat browsing without guessing ROM compatibility or promoting syntax-inferred formats.
- [x] Discover filename-derived game-title candidates across all platform catalogs, with platform-qualified keys and deterministic pagination.
- [x] Verify the SHA-256 checksums of all local title-catalog artifacts during cross-platform discovery, without requiring full cheat-bundle hashing.
- [x] Add an optional, read-only Linux/egui desktop explorer with original game/source/code browsing, copy-on-demand, provenance inspection, and responsive background loading.
- [ ] Complete desktop visual/layout and live Wayland interaction QA, then polish the explorer for everyday use.
- [x] Implement SNES Game Genie / Pro Action Replay decoding, plus provenance-conscious syntax-only interpretation for unlabeled codes (over 66,000 fields interpreted); publish an audited coverage report with source-join semantics tracked separately.
- [ ] Add console-specific codecs beyond SNES; emulator integration remains owned by its separate projects.

Keep the README about the project; keep work tracking here.
