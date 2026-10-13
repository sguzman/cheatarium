# Roadmap and acquisition queues

**North star:** a huge, pleasant, searchable collection, where original cheat files and meaningful game/edition/device relationships are both preserved.

## Rights-review acquisition scorecard (2026-10-12)

The reproducible [acquisition-priorities report](../generated/v1/reports/acquisition-priorities.json)
ranks the **39 existing link-only catalog IDs** by **distinct, unarchived Git blobs
with cheat-labeled original filenames that occur in only that source**, rather
than counting mirrors or unrelated patches as new game cheats. The tie-breaker
is total cheat-labeled unarchived distinct bytes, then exclusive blobs of any
kind, then source ID. Platform-level counts and up to three source path
witnesses per collection identify the best places to investigate contributor
licenses, not necessarily the best collection to redistribute.

Regenerate offline with `python3 tools/build_acquisition_priorities.py --write`,
and verify with `--check`. The report independently reconciles its pinned
blob counts against both previously published external overlap reports.
It is included in the SHA-256 distribution and rebuilt/verified in CI.
**All entries remain rights-unresolved**. No external cheat or patch payload
is fetched, copied, or marked verified. Exact source-byte identity does not
establish distinct games, valid codes, author rights, or ROM compatibility.

## Acquisition priority: BULK FIRST (2026-10-11)

Entire licensed/pinned game-cheat collections are the default unit of acquisition. Expand complete platform coverage before single-title work. Evaluate progress by new source files and new filename-derived game candidates (with duplicate and build ambiguity disclosed), never by commit count. Do not hand-curate isolated titles while material bulk coverage opportunities remain. Individual-title sources are a final gap-filling stage, justified only by measured coverage gaps. Rights-unresolved external data remains link-only until it can be redistributed; it must not be presented as acquired files.

At the pinned Libretro revision, the first mapped expansion imported **835 `.cht` files** from MSX, MSX2, FinalBurn Neo arcade, DOS and Atari 8-bit. A second whole-directory pass imported the remaining seven upstream collections, led by **3,683 ZX Spectrum +3 files**, plus ChaiLove, PrBoom, PuzzleScript, TIC-80, Thomson MOTO and Wolfenstein 3D (**3,698 cheat files; TIC-80 also contains one README that is intentionally not counted as a cheat**). These preserve source-system identities, including computer, arcade and engine categories, without pretending they are console ROMs. The automatic importer committed and validated both complete batches (4,533 new files). This completes discovery coverage of the **51 source directories** at the pinned Libretro snapshot; it does not complete console coverage outside Libretro.

## Non-overlapping CC BY-SA Libretro-fork acquisition (2026-10-12)

Pinned `BitHeX0R/Cheat-database` tree `837bc5b7` contains 28,278 native `.cht` files; the pinned upstream Libretro tree contains 27,915. A Git blob identity audit found 26,081 identical path-and-byte occurrences, 1,888 paths absent from the base source and 309 paths with different bytes. Rather than copying thousands of renamed duplicates, the new automated importer selects **all 1,125 source occurrences whose exact Git blob SHA is entirely absent from the pinned Libretro corpus** (**1,113 distinct previously absent Git blobs**). It preserves their original paths and byte-identical LICENSE/README in a separate archive source. Code authorship rights, physical game identity, code effects and version compatibility remain unverified. Two combined MSX-family folders cannot honestly be assigned exclusively to MSX or MSX2; they are indexed as advisory `msx-family`.

The importer finished successfully and the exact-byte archive was committed; subsequent CI regenerated and validated the expanded source bundles. **1,125 original cheat-file paths plus two LICENSE/README notices are now archived**, increasing the verified archive total to **67,831**. The searchable catalog now contains **57,351** source/game-section records, **47,502** provisional game groups and **1,825,471** code-bearing entries across **59** platform bundles. None of these records proves gameplay compatibility.

## Metadata-only external bulk source discovery (2026-10-11)

Thirty-nine source catalog IDs (some splitting shared upstream repositories by platform) are catalogued *without* copying code: DuckStation's pinned PS1 source tree contains **4,497 cheat files and 138 patch files**, while Xenia Canary's pinned Xbox 360 tree contains **501 patch TOMLs** (not necessarily gameplay cheats). All indexes retain original Git blob IDs, sizes and paths in `sources/external-bulk/`, with deterministic local name search via `tools/external_bulk.py`. Further pinned catalogs cover 4,710 PCSX2 PS2 PNACH patch files, 677 r0ah VitaCheat files, 119 ShumnoT VitaCheat files and 387 Dreamcast Markdown code documents. Additional pinned catalogs cover **2,582 native Switch cheat/patch paths** (including 66 IPS binary patches) plus **257 Nintendo 3DS title-ID cheat files** from ADEMOLA200's mixed-source mod database and **812 ASM/Markdown/IPS/PCHTXT references** from RoGamer97 across five Nintendo platforms. The Switch database contains 1,927 native cheat paths with 1,741 distinct title/build labels, **1,704 not present in the pinned NXCheats filename list**; these are source label differences, not verified new titles or effects. The upstream MIT grant does not establish the rights of every credited modding contributor, and RoGamer97 provides no root LICENSE. An exact-Git-blob overlap audit reports **the newly recomputed count of distinct original file bodies among these 42,718 references**, **the audited number of repeated-path excess occurrences** and **the audited number of blob hashes shared between cataloged source IDs**. This is byte identity, not unique games, effects, or verified support. The checksum-verified report at `generated/v1/reports/external-overlap.json` is regenerated from original pinned metadata on every CI build. These **42,718 link-only entries are excluded from archived-file and searchable-cheat totals** until explicit original-author reuse rights are established. DuckStation's upstream LICENSE licenses processing scripts under MIT but explicitly reserves cheat/patch author copyrights; Xenia has no repository-wide LICENSE. Never silently promote their file metadata into imported cheat payloads. Further pinned references cover **2,687 PS3 NCL Artemis/RPCS3 converted patches**, **239 extensionless RPCS3 patch files**, one **8.7 MB PSP CWCheat cheat.db** and one **42.98 MB Nintendo DS usrcheat.dat**. The PS3 sources reuse community-derived material and upstream 'confirmed' labels are not Cheatarium gameplay verification. The PSP/DS databases are each *one file*, not one game or one cheat; their code payloads are not copied. The linked-only Switch bulk inventory additionally covers **5,278 pinned JSON source files** in HamletDuFromage's daily mirror (2,501 gameplay title files, 2,245 GBAtemp mirror files and 532 graphics source files). All 2,245 GBAtemp paths overlap a gameplay source title ID, with 1,969 identical original Git blob IDs; these are file occurrences, not 5,278 independent games or verified codes. The README credits external authors and no root redistribution LICENSE was located, so code remains link-only. Three more pinned, metadata-only catalogs add **3,136** native Atmosphère Switch text cheats from ibnux, **5,861** Switch cheat/graphics JSON records from the sthetix multi-author mirror, and **628** 3DS CTRPF title-ID files from JourneyOver. None provides a repository-wide grant sufficient to cover the explicitly credited community cheat authors. The expanded exact-blob audit finds **the audited number of** distinct bytestring Git hashes across **42,718** references, with substantial overlap in the Switch mirrors. No code payloads are copied. The independent `generated/v1/external/` exports now package all 42,718 pinned original Git paths as deterministic, SHA-256-manifest-checked, read-only external browse records for the Rust `cheatarium-query external` CLI. These references remain outside the main console cheat bundle counts and never expose the rights-blocked code bodies.

## External-source byte overlap against preserved archive

A source-manifest-based, deterministic report at `generated/v1/reports/external-vs-archive-blobs.json` compares every pinned metadata-only external Git blob SHA-1 with all actually archived original source file Git blobs. It distinguishes file occurrences from distinct byte hashes, reports exact-byte matches and unmatched candidates per external source and platform, and preserves bounded witnesses of the underlying archived source path. This is a **bulk acquisition priority tool**, not a filename matching, permission grant or proof of unique game/effect content. It is regenerated from the two authoritative pinned source registries, checked during CI, and included in the versioned SHA-256 distribution manifest.

Two source-native PNACH libraries now extend the PS2 link-only inventory: **5,483** files from the pinned DAOWAce consolidated collection and **579** from its xs1l3n7x upstream conversion set. DAOWAce explicitly traces content to Ultimate PCSX2 Cheater, CheatDevice, legacy and forked community codes; neither pinned tree provides a redistribution license covering all original contributors. The new paths therefore remain pinned filename/hash references without cheat bodies. Their CRC-derived title identifiers, structural audit claims, and patch counts must never be mistaken for verified ROM compatibility or new game/effect totals.

The separate pinned native PS2 PNACH CRC/serial-label audit at `generated/v1/reports/external-ps2-crc-labels.json` now compares strictly parsed CRC-only, serial+CRC and CRC-prefixed filename labels across official PCSX2, DAOWAce and xs1l3n7x. It explicitly rejects malformed or scientific-notation CRC filenames rather than guessing, measures pairwise label overlap, and includes small lexical examples. Sharing a CRC is **not** evidence of shared game identity or executable compatibility; even an identical serial+CRC source label is not a validated ROM fingerprint. The report never imports the upstream code bodies.

## Strict native Switch source-label overlap across archives and metadata-only references

The automated `generated/v1/reports/switch-external-title-build-coverage.json` compares 16-hex Switch title/build-ID pairs in archived Sharkive and NXCheats with independently pinned **Ademola** and **ibnux** native Atmosphère file paths. It reports distinct pairs shared by the imported and external datasets, per-source unmatched pairs and native path repetitions. The **HamletDuFromage** and **sthetix** JSON mirrors are compared separately at title-ID-only scope, since their filename paths cannot establish build identifiers; RoGamer97 documents and patch formats remain outside native-cheat pair counts. The audit reconciles every linked file occurrence, checks rights remain blocked, runs in CI, and is checksummed in the distribution. Neither 16-hex filename labels nor agreement of source labels verifies games, effects, build compatibility, or permission to copy historical codes.

## Strict 3DS native cheat title-ID filename coverage

The deterministic `generated/v1/reports/3ds-external-title-id-coverage.json` compares Sharkive's **644 archived 3DS native title-ID cheat file labels** against the **257 pinned Ademola** and **628 pinned JourneyOver** metadata-only 3DS native file paths. It retains original contributor source counts, duplicate label occurrences, exact archive/external title ID overlaps and source-only labels. The **53 RoGamer97 3DS Markdown/ASM documents** have no verified native title-ID filename mapping and are counted only as unmatched structured-source documents. The report is produced solely from pinned filename inventories, validated in CI and checksummed in the v1 distribution. It cannot establish unique games, effect equality, compatibility or author reuse permissions.

## Automated bulk-coverage comparison

The version-pinned Nintendo Switch acquisition now includes bad1dea's complete NXCheats native source-text collection (2,992 build-ID-named cheat files, 3,155 total source originals including accompanying data and notices). A CI-generated Switch source overlap report compares Sharkive and NXCheats by exact native title/build-ID pairs and tracks repeated paths and nonstandard identifiers. It does not claim unique games, equivalent effects or tested operation.


The machine-generated `generated/v1/reports/ps2-bulk-source-coverage.json` compares the complete original root670 source game headings, NTSC and PAL ZIP titles, and CRC-labeled PNACH conversion titles using conservative lexical matching and an explicitly more relaxed region-label comparison. It records overlap and newly unmatched **source-name candidates**, not independently established new games. The CI build regenerates and checks it whenever it rebuilds the index; no manual curation or title choice occurs.

## PCSX2-ready bulk conversion (2026-10-11)

The pinned `sashkinbro/EmuCoreX-Cheat` source `e1803c3b` exposes **473 serial/CRC-labeled PCSX2 PNACH originals** derived from the GPLv3 root670 CheatDevice collection. Import the *entire* folder, keeping the original `gametitle`, effect headings, GPL and contributor notice. These files improve ready-to-load native emulator coverage, but overlap raw root670 codes; do **not** present the conversion as 473 independently discovered effects or tested compatible ROMs.

## PS2 ZIP-pack bulk expansion (2026-10-11)

The maintained GPL-3.0 CheatDevicePS2 fork `israpps/CheatDevicePS2` includes four large pinned original source ZIP packs. The complete source audit (Git revision `77b453ad`) confirmed **7,244 native ZIP members**: 4,013 original `.TXT` mastercode files plus one README, 1,880 NTSC `.CHT` game-cheat sources, 1,349 PAL `.CHT` sources, and one widescreen omnibus text file containing multiple game sections. Acquire *every* ZIP byte-identically with the complete contents and original license/README. Keep per-member original ZIP path, CRC32, pinned ZIP Git blob hash and computed extracted-byte hash. Index all 7,243 native code-bearing member files under PS2 while retaining source-declared sections and region labels; never count mastercodes as game-specific gameplay effects or confuse source sections with unique titles. No manual title selection or verification claims. Workflow success required before counting as archived.

## Console bulk acquisition — completed in this pass

- [x] PS5: etaHEN pinned GPLv3-original-source snapshot `ef295511`, **75 PPSA cheat files representing 56 original title IDs**; native JSON/SHN/MC4 and original project notices retained, distinct from already archived PS4 GoldHEN files.
- [x] PS Vita: rinCheat pinned GPLv3 source `d17fd2a7`, **58 original files across 57 Vita title IDs** (7 runtime-memory code sources, 51 save-editor Lua sources); no Lua is executed by the indexer.
- [x] PS2: root670 CheatDevicePS2 pinned GPLv3 project `7d9c525a`, complete **6,375,204-byte source database** containing **1,483 original source game sections** and **200,941 native hexadecimal code lines**. Its single original data file is preserved unchanged; the indexer deterministically exposes **1,483 separate source-game records**. Device-code group counts must not be confused with hex line counts or empirical cheat verification.
- [x] Archive validation, index generation, distribution integrity and offline-client smoke tests completed for all three imports.

**Standing rule:** all further acquisitions are bulk-first. The next target is entire eligible console collections that substantially improve breadth and title coverage, particularly under-covered Xbox 360, GameCube/Wii, PS2, Switch and Vita libraries. Keep rights-unresolved sources link-only and do not report their payloads as acquired. Manual single-game work is reserved for the final coverage-gap stage.

## Full-source converted PS3 bulk acquisition (2026-10-11)

The successful automated `mohasi/game-cheats` PS3 import preserves its exact MIT-noticed snapshot `08f2c50c`: all **4,343** compiled native plugin text files, **13,343** editable source/alias text files and **2** source notices, **17,688 original files** total. It is a converted, reformatted **derivative of already archived Artemis PS3**, not 17,688 original newly discovered games or codes. Keep all source aliases, author attribution, original scores and raw formatting unchanged, and expose only compiled serial files as source-native, **unverified** named cheat entries. Upstream score values are not gameplay tests conducted by Cheatarium. The automatic importer committed originals and attribution; CI rebuilt and validated the PS3 bundle. It adds **4,343** source records and **86,831** code-bearing index entries, all unverified Artemis-derived occurrences. Historical original cheat-author rights may vary despite the upstream MIT grant.

## Collection pipeline

1. **Discover** a source; identify platforms, file formats, size, provenance, and licenses.
2. **Clear** reuse rights or mark the source link-only.
3. **Acquire** a pinned snapshot (start with bounded batches to audit the pipeline).
4. **Inventory** imported files and validate integrity.
5. **Normalize** into per-game curated records without discarding raw material.
6. **Verify** codes against actual game builds and supported emulators.
7. **Publish** searchable indexes, then integrations/exporters.

## Acquisition status (2026-10-11)

**66,704 archived original files including source notices and companion data from 80 pinned collections**. The indexed distribution contains **56,226 source/game-section records across 58 platform bundles**, **46,802 unverified source-derived candidate game groups**, **1,819,305 code-bearing index entries** and **21,322 native memory entries**. An entire 2,992-file native Switch NXCheats collection, plus 163 source companion and license originals, was newly bulk-imported from a pinned MIT repository; this does not represent 2,992 distinct games or verified effects. Exactly 34 Dolphin originals remain unassigned. Original historical author rights and build compatibility are not independently confirmed. No real-world cheat execution verification is claimed.

## Platform queue

- [x] Register console/handheld platform IDs.
- [x] Start Libretro NES and SNES proof-of-concept imports.
- [x] Import 39 Libretro console/handheld source collections with pinned per-file provenance (23,382 files, 36 canonical console bundles).
- [x] Bulk-import the 12 remaining mapped Libretro directories (4,533 original `.cht` files), completing all 51 cheat directories at the pinned revision; preserve computer, arcade and engine classifications distinctly from consoles.
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


### Additional PS2 widescreen staging inventory (2026-10-12)

The pinned [PS2-Widescreen/pnach-DB](https://github.com/PS2-Widescreen/pnach-DB/tree/91902dd63b67b7c8e2fcdf71b165b8e21f415346) Git tree contains **3,632 .pnach patch files** in the `PNACH/` (232) and `PNACH_WITH_ID/` (3,400) directories. All 3,632 source paths and Git blob IDs are now separately indexed as metadata-only rights-blocked PS2 external references. These staging and widescreen patches are **patches, not counted as independently confirmed gameplay cheats**. The README states that some are awaiting CRC, ELF, or master-code metadata; it does not establish game/build compatibility. No root LICENSE or blanket original-author redistribution permission was found. The existing PS2 filename-CRC audit now compares all four pinned external PNACH collections and the acquisition priority scorecard discounts exact-byte overlap. Nothing in the staging collection was added to the archived cheat totals.


### PS1 derivative fork delta (2026-10-12)

The pinned [crashGG/ps1.chtdb](https://github.com/crashGG/ps1.chtdb/tree/a25a878aac35b3c4c4c9aab43ec266b900259b44) source has **4,485 native PS1 .cht** occurrences. Exactly **4,217** already have identical Git blobs in the pinned DuckStation external catalog; the remaining **268 different-byte original paths** (263 cheat-labeled, 5 patch-labeled) are retained as a **separate link-only delta**, not as a redundant 4,485-file mirror. All retained paths also exist in DuckStation but with differing bytes: these may be changed or older variants for the same titles, not 268 distinct games or functional cheat effects. The fork's upstream LICENSE explicitly reserves cheat/patch rights to original authors (MIT applies only to processing scripts); no copied code bodies or asserted game compatibility. The baseline exact-blob filter is reproducible from both immutable source snapshots.


### Nintendo DS and PSP multi-format database witnesses (2026-10-12)

[szTheory/NDS-Cheat-Databases](https://github.com/szTheory/NDS-Cheat-Databases/tree/0173af14d33e2c045e7c5e30c71d369624a33020) retains **nine original compiled source-file occurrences**, comprising eight byte-distinct Git blobs (the two `cheat.db`/ `cheat_EN.db` files are exact bytes), across Action Replay outputs for multiple DS devices/emulators. The largest individual XML file is **103,408,603 bytes**. The collection's README credits a separate DeadSkullzJr source; the root AGPL text cannot alone settle the original individual code authors' rights. [Saramagrean/CWCheat-Database-Plus-](https://github.com/Saramagrean/CWCheat-Database-Plus-/tree/5ab71718f01810e7980d29a48b4755463233c55b) adds one **8,922,567-byte** PSP `cheat.db` reference. Its README credits community contributions and provides no blanket license. Both are indexed as searchable, pinned, **link-only whole-database file references**, not extracted game IDs or imported code effects. Alternative formats must not be added up as distinct game coverage.


### Next bulk collection discovery leads (not imported, 2026-10-12)

Beyond the 27 pinned external catalogs, four further major source candidates were inspected and added to `sources/candidates.json`: **Pugsy's MAME** XML/JSON releases (including an 0.279 release announced 2025-07-27; historically thousands of XML cheat files), **CheatBase** (67,366,912-byte multi-console SQLite database with a separate cheatCredit author field), **pacnpal's Mupen64Plus N64** (1,281,051-byte source `mupencheat.txt`), and a **MAME 2010** 24,981,772-byte `cheat.zip` distribution. The size of a compiled database/ZIP is not a game count or number of new cheat effects. All four are discovery leads, not archived originals or external Git source catalog entries. Original-code authorship and data-redistribution terms remain unverified; the two MAME candidates can be historically derivative. Prioritize archive format inspection, byte-overlap auditing, and explicit rights evidence, then import only reusable and attributable bodies. This project remains in **bulk acquisition**, not individual-game gap filling.


### MAME and derivative PS2 bulk audits (2026-10-12)

Three further pinned, link-only source catalogs preserve original Git tree identifiers and exact file sizes **without copying cheat bodies**. `libretro/mame2010-libretro` contains a 24,981,772-byte `metadata/cheat.zip` that exactly matches the byte hash of the earlier `galisteogames/mame2010_cheat` fork: the byte-identical mirror is not registered a second time. `libretro/mame2003-plus-libretro` supplies one separate 7,810,253-byte `metadata/cheat.dat`. Both compiled archives are under **`arcade-mame`**, not FinalBurn Neo; source LICENSEs for emulator code do not automatically clear the many independently authored cheat bodies. Their member counts, game identities and practical cheat effects remain unmeasured. `shadowninja826/pcsx2_pnach_cheats` has 628 pinned original PNACH file occurrences: **579 duplicate a previously pinned PS2 source exactly**, **one additional file is empty**, and **48 nonempty files have distinct bytes**. The 48 are indexed as a rights-unresolved delta, not imported. The strict PS2 external filename/CRC audit now includes five catalogs. These measurements are bytes and filename claims, never 48 established new games or working cheats.


### More Libretro-family mirror exhaustion, without double counts (2026-10-12)

Compared pinned legacy `gcmcom/libretro-database` and `Smoker1/RetroArch-174-and-above-Cheats` to the immutable original Libretro, 2019 Emupedia and archived BitHeX source bytes. GCMCom is **10,861/10,872** exact-blob matches to Emupedia; only **11** `.cht` file occurrences are outside the three compared source byte sets. Smoker1 offers 518 `.cht` files, but only **75 occurrences / 66 distinct blobs** outside those sets. Combined: **86 file occurrences / 77 Git blobs** for further rights and provenance review. This is not a total cross-platform archive novelty audit; other preserved sources might share bytes. The [metadata-only report](../research/retroarch-mirror-source-gaps-v1.json) and audited pinned revisions avoid copying cheat bodies, claiming authorship, or misleadingly counting the entire old fork as a new bulk collection. An unexamined 98-file INI group in Smoker1 remains a separate future acquisition candidate.


### FinalBurn Alpha and N64 Mupen64Plus bulk witnesses (2026-10-12)

The pinned [Smoker1 RetroArch 1.7.4+ cheat tree](https://github.com/Smoker1/RetroArch-174-and-above-Cheats/tree/d07b731562d3a68d089dc0a1ba62311b65a1d312) contains 98 FB Alpha Official Cheats/*.ini file occurrences totaling 573,596 source bytes, but only **45 distinct Git blobs** due to 53 same-byte clone/alias occurrences. They are searchable through the separate arcade-fba platform; no FBA-to-FBNeo or MAME compatibility is inferred. The publisher's README cites these as preexisting official FB Alpha cheats and explains using/adapting them in RetroArch, not an author-wide grant. [pacnpal's Mupen cheats](https://github.com/pacnpal/mupen-cheats/tree/3dd74ad738741979aaf5ac5fda2611fe0ad5c53a) adds one 1,281,051-byte mupencheat.txt file under n64. The README explicitly describes newer GoldenEye 007, Perfect Dark and Donkey Kong 64 GameShark modifications mixed with preexisting Mupen and old FAQ codes; redistribution rights of the older code corpus were not shown. Both sources are pinned, counted by Git blob SHA-1, and searchable **as read-only link-only file references**; no cheat bodies, emulators or test-ROM results are included. The larger RetroArch CHT fork audit remains a separate metadata-only candidate and is not conflated with these 98 INI files.


### Seven consumer-searchable historical RetroArch fork gap partitions (2026-10-12)

The previously audited [GCMCom / Smoker1 exact-byte gap report](../research/retroarch-mirror-source-gaps-v1.json) is now integrated into the external search catalog, not just a research footnote. In the immutable original source trees, GCMCom has 10,872 .cht occurrences and Smoker1 has 518. Compared solely with three pinned cheat byte corpora (Libretro, Emupedia and BitHeX), only 11 GCMCom occurrences and 75 Smoker1 occurrences were not exact-byte matches: **86 file occurrences / 77 distinct Git blobs**. Those candidates are partitioned into GBA (2), NES (8), SNES (1), Smoker1 FBNeo (32), Dreamcast (12), NAOMI (9), and ten other RetroArch core folders (22). Some paths in the mixed 22-record bucket refer to entirely different systems: the multi-retroarch label is an explicit mixed-source grouping, **not** a universal emulator format or evidence of compatibility. Each candidate retains the original upstream path, Git SHA-1, size, and frozen commit URL. The source importer cross-checks every exported record against both the local three-corpus gap inventory and the original complete pinned Git tree before refreshing. It **does not** copy cheat text, claim rights, or assert these bytes are absent from every other archived collection.

### Newly pinned bulk-source scouts (2026-10-12)

The reproducible [metadata-only bulk scout](../research/bulk-source-scout-v1.json) and
`python3 tools/audit_bulk_source_scout.py --offline-check` record **691 original
file occurrences across four pinned sources / 487 distinct Git blobs** without
copying the cheat code, archive bodies or executables. These are
**research candidates, not external consumer-search entries or archived files**.

- **Arcade:** Eignar17's historic FinalBurn Alpha/Kawaks files contain 253 FBA INIs
  plus 368 legacy DAT files (621 occurrences, 420 different Git blobs).
  None shares exact blob bytes with the 98 pinned Smoker1 FBA INI occurrences;
  legacy DAT and INI represent different native formats, possibly equivalent effects.
  The original archive README attributes prior FBA work and lacks blanket code-author
  redistribution permission.
- **Nintendo:** Riyoway's recently published SQLite / JSON / JSONL / Wii U XML
  releases contain seven selected output blobs totaling 191,660,391 bytes.
  Its README **claims**, but we have not independently validated, 5,327
  game/variant records and 117,861 cheats. Different exports must not be summed
  as distinct data. No root redistribution license was found.
- **Sega CD:** Slade1972 has 51 per-title MiSTer ZIP occurrences, 48 distinct
  Git blobs. A GPLv3 LICENSE is present, but the README explicitly distinguishes
  cheats found by other people; ZIP internals and code author rights still
  need inspection.
- **Original Xbox:** SkillerCMP's Xemu source has 12 native cheat text files.
  Unrelated file replacements and binary assets are excluded; no root
  redistribution license was located.

Next: investigate actual member/data-format structure and source lineage, audit
against **all** archived/pinned blobs (the arcade comparison above is Smoker1-only),
then pursue rights clearance before moving eligible content into the original
archive. See the four matching entries in `sources/candidates.json`.

### Deferred idea: headless cheat-effect verification

A future, separate research phase may test opt-in cheats against **user-supplied,
locally possessed game images** through a headless emulator runner (possibly
Starbyte for SNES). Capture exact ROM SHA-256, core/build version, code format,
initial state, baseline/control run, modified run, memory or game-state
observations, and evidence of a repeatable difference. A changed byte alone
is not a verified intended cheat effect. No ROMs or test saves belong in
Cheatarium; **this is not part of the current acquisition pass**.

### Whole-corpus novelty audit for four pinned bulk scouts (2026-10-12)

The reproducible `generated/v1/reports/bulk-scout-exact-byte-gaps.json` now
compares all **691** selected original file occurrences (**487** byte-distinct
Git blobs) against every file in the preserved archive's source manifests and
all previously indexed link-only external catalogs. This is substantially
more rigorous than the earlier Smoker1-only FBA comparison: exact-byte matches
are separated from byte-only gaps, with per-source bounded path witnesses.
Every code body remains rights-unresolved and outside the indexed playable
cheat archive. An exact-byte gap is *not* an established new game, effect,
compatible build or redistribution grant. Regenerate with
`python3 tools/build_bulk_scout_overlap.py --write`, verify with `--check`;
CI independently recomputes and checksum-manifests the report. Next acquisition
step remains clearing authorship and inspecting compressed database/ZIP formats
before importing eligible original code bodies.

### Four audited bulk scouts now searchable by external consumers (2026-10-12)

The previously pinned 691 original file occurrences / 487 byte-distinct
Git blobs from Eignar17 (FinalBurn Alpha/Kawaks), Slade1972 (Sega CD MiSTer),
SkillerCMP (original Xbox Xemu) and Riyoway (Nintendo database exports) are
now packaged as nine rights-unresolved **external metadata-only partitions**.
Classic FBA INIs (253) are kept separate from legacy arcade DAT (368);
Nintendo's SQLite/JSONL mixed-platform exports (2) stay separate from native
NES (1), Switch (1), 3DS (1) and Wii U (2) outputs. Sega CD ZIPs (51) and
Xemu text files (12) retain original paths and formats. No contents of ZIPs,
compiled databases or cheat text have been copied into preserved archives.

There are now **43,409 link-only external references** in **48 source
catalogs across 26 external platform labels**. The pre-onboarding whole-corpus
byte-gap report continues comparing scout bytes against the original 42,718
older external references and archived originals. This intentionally excludes
the nine newly searchable source partitions from comparing with themselves,
so the historical **487 previously untracked blobs** remain auditable.
All nine partitions are checksum-verified, source-path searchable offline,
and explicitly not gameplay-verified or cleared for cheat-body redistribution.

### Compiled code overlap: PSP CWCheat, Nintendo, Sega CD (2026-10-13)

The source-verified, ephemeral [compiled source identity audit](../research/compiled-source-identity-audit-v1.json)
compares original data **without exporting original cheat text, individual source
identifiers, hashes of code blocks or ZIP member filenames**. The GitHub Actions
job validates each pinned Git blob SHA-1 and uploads aggregate counts only.

- **PSP derivative overlap:** Saramagrean and ApelFactory CWCheat databases
  share **2,642 exact game-serial labels**. Saramagrean has another **36**
  label values; all ApelFactory labels occur in Saramagrean. Comparing
  each source-declared serial with its *exact normalized* `_L` command block
  (ignoring header labels), **75,036 code-block pairs appear in both**.
  Saramagrean has **2,281** additional exact pairs and ApelFactory **1,586**.
  Together they form **78,903** distinct serial+text pairs, not the sum of
  either collection's group counts. Exact equal bytes are stronger evidence
  of redundant content, but *not* guaranteed compatible game builds, unique
  effects, verified authorship or reuse rights. Original files remain links.
- **Nintendo canonical data:** all four platform JSON exports reconcile
  exactly with SQLite: **5,327 source game/variant rows and 117,861 cheat
  rows**. Per-platform game/cheat rows are Switch 2,491/56,476; 3DS
  644/12,265; NES 2,180/42,172; Wii U 12/6,948. Source-native text
  repetitions within the same ID label are separately measured, not
  destructively merged. The multiple export formats are **not additional
  independent collections**.
- **Sega CD:** the 48 byte-distinct parent ZIPs contain **385 file-member
  occurrences**, but only **344 distinct member byte hashes**, leaving
  **41 repeated member occurrences** across source ZIPs. The ZIPs contain
  364 `.gg` and 21 text files, and the textual "credit" marker was not
  observed in those text members. That observation is **not** a claim that
  credit or contributor ownership is absent; original rights remain blocked.

Next: scrutinize the 8,837 XML members of pinned MAME 2010 at per-file cheat
entry level, then prioritize the most useful, author-cleared source families.
Headless execution testing and ROM/build matching remain deferred.

### MAME 2010 XML cheat internals quantified (2026-10-13)

The [pinned, original-Git-blob-verified MAME 2010 XML audit](../research/mame-xml-structural-audit-v1.json)
reads the original 24,981,772-byte link-only ZIP ephemerally, parses every
member without keeping XML/cheat bodies, and publishes **aggregate counts**.
All **8,837 XML files** parsed with the declared `mamecheat` root and version
1. There are **8,837 distinct filename stem labels**, but only **4,108
different full XML file bodies**; hence **4,729 repeated file-byte occurrences**.
Across all files there are **106,805 `cheat` elements**, but only **33,400
distinct exact serialized `cheat` XML subtrees**, so **73,405 occurrences
repeat a subtree's exact bytes**. The source also has 89,243 `script` and
145,757 `action` element occurrences. Repeated XML bytes are highly
significant for source-level de-duplication and mirrored clone aliases.
They do **not** prove game-equivalent behavior, source-author rights, or
game-release compatibility across different arcade sets.

Acquisition priorities: separate MAME filename counts, full-file byte
identity, subtree content identity, and validated game identities rather
than summing the 8,837 aliases or 106,805 source elements as unique playable
cheats. The ZIP remains a rights-unresolved **link-only** collection. Next
work: identify earlier original cheat authors and overlap against other
MAME versions before any new original-body import.

### MAME 2003-Plus native DAT versus MAME 2010 XML coverage (2026-10-13)

The [original-source-verified MAME driver-label audit](../research/mame-dat-xml-driver-labels-v1.json)
compares a pinned 7,810,253-byte MAME 2003-Plus `cheat.dat` against
the **8,837** member-name stems of pinned MAME 2010 `cheat.zip`.
Parsing the [native colon-separated `cheat.dat` code record grammar](https://github.com/libretro/mame2003-plus-libretro/blob/master/src/cheat.c),
the older DAT contains **28,598 syntactically recognized code record
occurrences**, of which **27,477 are distinct exact source lines**, across
**546 distinct native driver labels**. The remaining nonempty text is not
classified as cheat code by this strict parser. Of those 546 DAT labels,
**493** are also XML filename labels, leaving **53 DAT-only** labels and
**8,344 XML-only** labels in the compared sources. This is a comparison
of historical driver-name strings, **not** a determination that 53 new
playable games or 8,344 newly discovered cheat effects exist, or that
matching labels refer to compatible ROM versions. All cheat bodies
remain upstream-only because original author redistribution rights
have not been established.

Bulk-acquisition implication: preserve the DAT separately as a historical
lineage, prioritize the 53 DAT-only driver labels for future rights and
content coverage investigation, and avoid presenting 2003 DAT and 2010 XML
counts as independent unique games or effects.

### Nintendo-specific bulk acquisition: GameCube and Wii U source gap (2026-10-13)

Newly pinned source-native links are now searchable in the offline external
browser. **GameCube:** LeumasPlays' three regional WiiRD original ZIPs
(NTSC-J, NTSC-U, PAL; aggregate source-container bytes **1,310,887**),
compiled from gc-forever contributors. They may hold far more codes than the
three ZIP filenames suggest and must be inspected at the original member
level; a file count of three is **not** a coverage estimate. The compiler
reports having tested only a handful of NTSC-U codes; no independent
per-code compatibility conclusions follow. **Wii U:** nine native XML
source documents from KatsuMaki1 (46,756 source bytes) plus four JSON source
documents from Vincent Coding's uCodes (33,633 source bytes; three distinct
Git blobs due to a same-byte alias). The Wii U source documents include
third-party credited code authors; a root website MIT license does not
certify reuse of contributed cheat data. All **16 source-file occurrences,
15 distinct Git blobs**, are original-blob pinned and currently
**link-only**. No original Wii U code or ZIP members have been copied to
Cheatarium's archived executable bundle. The external catalog advances to
**43,425 source-file occurrences / 51 link-only source catalogs**; native
Nintendo indexed code counts are unchanged.

Next: use bounded, read-only ZIP inspection to quantify GameCube cheat files
and compare source file aliases by exact member bytes; inventory Wii U XML
and JSON declared code entries and attribution indicators without publishing
individual cheat bodies. Track verified title/revision compatibility only
after original-author reuse rights and test evidence are available.

### GameCube WiiRD regional contents and Wii U native format counts (2026-10-13)

The [pinned source-verified Nintendo bulk member audit](../research/nintendo-bulk-member-coverage-v1.json)
inspected all three original WiiRD GameCube ZIPs: NTSC-J contains **8**
`.txt` original member occurrences, NTSC-U **138** and PAL **229**:
**375** total source text file occurrences, **375** distinct exact member-byte
bodies, and no exact cross-region member-name or byte aliases. These are
source-text files and are **not** 375 independently validated games,
revisions, or cheat effects. The compiler sources from gc-forever and
performed only limited personal testing; original-code reuse rights remain
unresolved.

The **nine original Wii U native XML** source documents contain **97**
`entry` elements, of which **88** carry nonempty `code` text, with
**90** entry occurrences bearing author credit labels (10 distinct
source-credit strings). The **four Wii U uCodes JSON** documents contain
**50** record occurrences, **48** with Cafe code text and **35** with
uGecko code text. Two original JSON source files have identical Git bytes;
do not count them as separate data. These are native syntax counts, not
confirmed unique games or per-region compatible cheats.

All measurements use verified pinned Git blobs, ephemeral extraction and
aggregate-only reporting. The GameCube ZIPs, Wii U XML/JSON code bodies
are **not** archived; 16 link-only parent source-file references are
searchable. Next priority is extracting safe ZIP member path/title labels
into a code-free per-game lookup (without original code bodies) and
documenting original-author reuse permissions for GameCube and Wii U.
