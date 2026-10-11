# Roadmap and acquisition queues

**North star:** a huge, pleasant, searchable collection, where original cheat files and meaningful game/edition/device relationships are both preserved.

## Acquisition priority: BULK FIRST (2026-10-11)

Entire licensed/pinned game-cheat collections are the default unit of acquisition. Expand complete platform coverage before single-title work. Evaluate progress by new source files and new filename-derived game candidates (with duplicate and build ambiguity disclosed), never by commit count. Do not hand-curate isolated titles while material bulk coverage opportunities remain. Individual-title sources are a final gap-filling stage, justified only by measured coverage gaps. Rights-unresolved external data remains link-only until it can be redistributed; it must not be presented as acquired files.

At the pinned Libretro revision, the first mapped expansion imported **835 `.cht` files** from MSX, MSX2, FinalBurn Neo arcade, DOS and Atari 8-bit. A second whole-directory pass maps the remaining seven upstream collections, led by **3,683 ZX Spectrum +3 files**, plus ChaiLove, PrBoom, PuzzleScript, TIC-80, Thomson MOTO and Wolfenstein 3D (**3,699 source files total**). These preserve source-system identities, including computer, arcade and engine categories, without pretending they are console ROMs. The automatic importer must commit and validate the second pass before its contents count as acquired. This completes discovery coverage of the **51 source directories** at the pinned Libretro snapshot; it does not complete console coverage outside Libretro.

## Collection pipeline

1. **Discover** a source; identify platforms, file formats, size, provenance, and licenses.
2. **Clear** reuse rights or mark the source link-only.
3. **Acquire** a pinned snapshot (start with bounded batches to audit the pipeline).
4. **Inventory** imported files and validate integrity.
5. **Normalize** into per-game curated records without discarding raw material.
6. **Verify** codes against actual game builds and supported emulators.
7. **Publish** searchable indexes, then integrations/exporters.

## Acquisition status (2026-10-11)

**33,272 original cheat files archived from 73 pinned source repositories**; **33,238 searchable across 44 console bundles** with **34 unassigned archived Dolphin INIs**. The published corpus contains **1,259,044 code-bearing source occurrences** and **665 indexed Nintendo 3DS original files**. The latest three sources add five native 3DS text files: FoxyXTN Settings-app CTRPF, three byte-identical Cracko298 Minecraft 3DS mob-cap files with distinct regional IDs, and one Cracko298 Minecraft 3DS save-control file. All source blobs matched their pinned Git SHA-1 values; the combined CI run passed source-inventory, Rust, index, distribution and consumer checks. Codes and compatibility remain untested; settings and save modifiers can be destructive. Console cheat acquisition, not unrelated app development, is the priority.

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
- [x] Acquire and index one SalustLab Klonoa 2 hard-mode PS2 PNACH and three PreachingPython Kingdom Hearts Final Mix PS2 PNACH variants (3x, 4x, diagnostic 100x), preserving original MIT notices and compatibility caveats.
- [x] Preserve 14 original igorciz777 TXR/Kaido controller PNACH files (MIT) and Saupernova13's released Budokai Tenkaichi 3 60 FPS PNACH (CC BY 4.0); exclude WIP experiments and retain differing upstream rights notices.
- [x] Acquire original Wii Gecko codes from e.the.real (3), 3DS Gateshark texts from four author-licensed FoxyXTN projects (4), and Sly 2 PS2 PNACH reimplementations from andshrew (5), retaining pinned source hashes and original licenses.
- [x] Archive and index 24 additional independently pinned cheat originals: 14 Wii Gecko INIs from five MIT-licensed sources, 8 3DS CTRPF texts from seven Apache-licensed sources, and 2 Need for Speed Underground PS2 PNACHs from xan1242, with exact original source paths and SHA hashes.
- [x] Archive and index 57 additional native cheat-bearing source files from 13 more repositories: 43 Wii Gecko INIs, 9 GameCube Gecko Markdown files and 5 Gran Turismo 3 PS2 PNACHs. Confirm original Git blob identity and run live source-lookup smoke checks; preserve external code-author credits and source-level rights limitations.
- [x] Archive and index 12 additional native cheats from six pinned original sources: 10 Wii Gecko INIs, one GameCube 007 Agent Under Fire code text and one 3DS Ice Station Z item-changer text. Preserve exact author files, original source licenses and revision-specific alternatives, and verify source lookups in CI.
- [x] Archive and index 11 native cheat sources from four further pinned repositories: 5 Wii Galaxy 2 INIs, 4 Wii Twilight Princess regional/revision INIs, 1 mixed-game GameCube Gecko author document, and 1 Ice Station Z 3DS test-code document. Keep original MIT/GPL/Apache notices and source author caveats; source search queries and Git blob identity validated.
- [x] Preserve 16 additional original native Wii cheat files from four independently pinned source repositories, including WiiWare plain-text Gecko files. Preserve per-revision filenames and original community credits; verify source hashes, rust tests, index and CLI queries.
- [x] Preserve one original PS3 NFS Carbon PowerPC Markdown cheat source and one original Ice Station Z 3DS CTRPF first-person code, with original license/input files, pinned Git SHA, untested source-build caveats and CI source-lookup smoke tests.
- [x] Preserve five 3DS native CTRPF originals across three newly pinned Apache-2.0 projects: one FoxyXTN Settings-app source, three byte-identical Cracko298 Minecraft mob-cap regional title-ID files, and one Cracko298 Minecraft save-control source. Retain original license/README, exact Git blobs and destructive-code caveats; publish and validate the index in CI.
- [x] Archive and index two distinct original GameCube Gecko files from shaigem and two distribution-path variants from 2gifts. Keep author labels and identical-code warnings intact; exclude separately credited source files until their author's reuse terms are confirmed.
- [x] Add one additional Exceen PSP CWCheat original from the pinned MHFU project, preserving the native transmog source and running code-field smoke tests.
- [x] Acquire and index one MIT-licensed Code Veronica X PS2 PNACH from fmil95 with its original game warning and author identity, plus one FoxyXTN 3DS Ice Station Z author-permitted WTW source with its no-SPDX-license caveat; validate original Git blobs and query provenance.
- [x] Add 16 original cheat-bearing files from four independently pinned upstream sources: six Made In Slovakia Ratchet & Clank PNACHs, seven xan1242 Sonic Unleashed PNACHs, two xan1242 NFS Most Wanted OPM demo PNACHs and Exceen's original PSP CWCheat database. Preserve source caveats, license, original hashes and search-client regression coverage.
- [ ] Resolve redistribution rights of the multi-author Ice Station Z CTRPF compilation before importing its code payload.
- [ ] Expand GameCube/Wii/Wii U and other missing console source collections beyond these additional authors, and acquire independent 3DS/Switch/Vita sources where original-code reuse rights permit.
- [x] Add Libretro Master System, Game Gear, Sega CD, 32X, Saturn, Dreamcast and SG-1000.
- [x] Archive 5,445 original GoldHEN PS4-oriented native JSON/MC4/SHN/XML files and 2,540 original Artemis PS3 NCL cheat files, retaining exact revisions, original authors and license notices.
- [x] Index original PS3 NCL, PS4 GoldHEN JSON/SHN/MC4/XML, and GameCube/Wii/Wii U Gecko/AR source records without fabricating decoded cheats or game compatibility.
- [x] Acquire and index 110 original PS2 PNACH cheat files from pinned CookiePLMonster, plus 67 PS1, 4 PSP, and 2 GameCube native cheat files with author attribution.
- [ ] Expand Sony coverage with additional independent PS2 and Vita collections; GoldHEN PS2-era title IDs are not native PS2 coverage. The 676-file r0ah and 119-file ShumnoT VitaCheat databases, and the official PCSX2 repository with 4,710 PNACH files, remain link-only pending contributor and redistribution rights review.
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
