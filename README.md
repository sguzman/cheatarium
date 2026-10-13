# Cheatarium

**A wonderland of console cheats.**

Cheatarium collects, preserves, organizes, and explains video game cheats: Game Genie and Action Replay codes, GameShark and other device codes, emulator cheats, button sequences, passwords, unlockables, and strange ways to bend a game to your will.

This is primarily a **console and handheld game** archive. The goal is a fun, searchable collection that respects the original community work and remains useful decades later.

## Explore

- **New Nintendo Wii acquisition:** Pinned original Git blobs for 29 region/revision-specific Gecko source files in Animal Crossing: City Folk/Deluxe (9), Metroid Prime/Trilogy (12), Metroid: Other M (3), and Taiko (5). They are public source references only; author-level rights and executable compatibility are unresolved.


- **Private Nintendo original-code acquisition:** `python3 tools/hydrate_nintendo_bulk.py --fetch` downloads the three pinned WiiRD GameCube regional source ZIPs and thirteen Wii U XML/JSON documents into `~/.local/share/cheatarium/nintendo-originals/` (or `$XDG_DATA_HOME/cheatarium/nintendo-originals/`). The result is **375 actual GameCube text cheats and 13 Wii U original source files** on the user's filesystem, with verified pinned Git SHA-1, checked ZIP member inventory, and a locally SHA-256-verifiable manifest. Run `--check` offline. The public repository and its packaged distribution do **not** contain these third-party cheat bodies, and no gameplay/ROM compatibility or contributor-wide redistribution rights are inferred. See [local Nintendo acquisition](docs/CONSUMERS.md#private-nintendo-original-source-cache).


- [Nintendo GameCube regional member lookup](research/nintendo-gamecube-wiird-member-claims-v1.json) — 375 original WiiRD ZIP member filenames, with NTSC-J/U/PAL regions, original container links and **no** cheat bodies. Search offline with `python3 tools/query_gamecube_wiird_members.py --query GZLE01 --region NTSC-U`. Source filenames do not verify game identity, execution or licensing.

- [Curated games](curated/) — human-friendly cheats grouped by platform and game, including versions and code formats.
- [Original archives](archive/) — unmodified, attributable imports in their original formats.
- [Historical Libretro preservation gap](docs/HISTORICAL-LIBRETRO.md) — pinned Emupedia-era byte-overlap audit, rights-limited.
- [New bulk-source byte gap audit](generated/v1/reports/bulk-scout-exact-byte-gaps.json) — four pinned source inventories compared against all archived originals and existing external metadata, without copying code.
- [Bulk acquisition priorities](generated/v1/reports/acquisition-priorities.json) — mirror-discounted, rights-blocked source-by-source exact-blob coverage rankings; not redistribution grants.
- [Pinned bulk source rights review](docs/RIGHTS-REVIEW.md) — documented original-author blockers, immutable upstream license/README evidence and next clearance actions.
- [Platforms](platforms/platforms.json) — stable platform identifiers, including console and handheld generations.
- [Sources](sources/README.md) — upstream repositories, attribution, licenses, and snapshots.
- [Adding cheats](docs/ADDING-CHEATS.md) — how new collections and individual cheats enter the archive.
- [Emulator consumers](docs/CONSUMERS.md) — read-only Rust client and offline search.
- [Desktop explorer](docs/DESKTOP.md) — optional Linux app to browse original cheat entries and provenance.
- [Versioned index](docs/INDEX-V1.md) — deterministic game/source bundles and compatibility rules.
- [Repeated cheat codes](docs/REPEATED-CODES.md) — source-preserving duplicate-text discovery without unverified game or effect merging.
- [Effect signals](docs/EFFECT-SIGNALS.md) — browse gameplay-effect descriptions by explicit language cues without unverified functional claims.
- [Effect reviews](docs/EFFECT-REVIEWS.md) — independently referenced reports and exact-build observations, separate from text search.
- [SNES code research](docs/SNES-CODES.md) — device decoding, revision-alternative evidence, audited coverage, and read-only source dossiers.
- [Distribution and integrity](docs/DISTRIBUTION.md) — SHA-256 manifests and verified consumer downloads.
- [ROM identity evidence](docs/ROM-IDENTITY.md) — read-only SHA-256 fingerprinting, optional local ZIP inventory scanning, and source-reviewed release claims.
- [Design](docs/DESIGN.md) — the boundaries and data model.
- [Roadmap](docs/ROADMAP.md) — systems, imports, and future features.

## Principles

1. **Preserve originals.** Keep imported files intact; make curated versions separately.
2. **Provenance is part of the cheat.** Every imported file must link back to an upstream source, pinned revision, and licensing information.
3. **A code is not universal.** Distinguish console, game, region, revision, emulator/device, and code format.
4. **Don't mistake collected for verified.** Imported cheats are untested until actually checked.
5. **Make room for the weird stuff.** Traditional cheat devices, secrets, glitches, passwords, and quality-of-life modifications belong here.
6. **No game images or firmware.** This is a cheat library, not a ROM or BIOS archive.

## External bulk reference catalogs (link-only)

- [Pinned external source metadata](sources/external-bulk/) — filename, blob hash and byte-count indexes for 4,497 DuckStation PS1 cheats, 138 PS1 patches, 501 Xenia Xbox 360 patches, 4,710 official PCSX2 PS2 patches, 796 VitaCheat sources from two databases and 387 Dreamcast source documents. Two further repositories add **2,582 Switch-native cheat/patch filenames plus 257 native 3DS title-ID cheat filenames** and **812 Nintendo console code/patch references** across Wii, GameCube, DS, 3DS and Switch. New pinned references add **2,926 PS3 community-derived patch files**, a single **8.7 MB PSP CWCheat database** and a single **42.98 MB Nintendo DS cheat database**. A further **5,278 Switch JSON cheat/graphics source paths** from HamletDuFromage's community mirrors (2,501 gameplay, 2,245 GBAtemp mirror, 532 graphics) are now pinned and searchable; 1,969 gameplay/mirror title files are byte-identical. Two additional PS2 collections add **5,483 consolidated DAOWAce PNACH file references and 579 xs1l3n7x original conversion file references**, each pinned and link-only; both contain community-derived cheat data without an established blanket author reuse grant. An additional **3,632 source-native PCSX2 widescreen-staging PNACH patches** are pinned from PS2-Widescreen/pnach-DB, with author-rights and patch compatibility unresolved. These are patch references, not asserted gameplay cheats. Additional link-only collections now cover **3,136 Atmosphère native Switch cheat files** from ibnux, **5,861 Switch cheat/graphics JSON documents** from sthetix and **628 Nintendo 3DS CTRPF title-ID files** from JourneyOver. These are independently pinned file occurrences, often derivative mirrors, not proven new games or effects. Archive overlap is also independently audited at [`external-vs-archive-blobs.json`](generated/v1/reports/external-vs-archive-blobs.json), showing which rights-blocked filenames point to bytes already archived without inferring reuse rights or code effects. Exact-blob audit: the pinned external catalog now contains **43,454 link-only original file references**. Distinct Git blob counts, repeated mirror paths and cross-source overlaps are rebuilt automatically in the [byte-identity report](generated/v1/reports/external-overlap.json). Git byte identity is not verified identity of cheats, games, or effects. These **43,454 upstream files are not archived**: rights are unresolved and some patches are not cheats. Source links are read-only references, not independently tested compatibility evidence. Use `cheatarium-query external --db generated/v1 --platform ps1 --title SLUS --json` for checksum-verified offline filename search, or `python3 tools/external_bulk.py --search "SLUS-" --platform ps1` to search the original source inventories.

**Bulk Libretro-fork delta:** The pinned CC BY-SA 4.0 source-delta importer preserved **1,125 cheat-file occurrences containing Git blobs absent from the archived Libretro baseline** (1,113 distinct new blobs), plus the original LICENSE and README. It does not re-import the more than 26,000 already identical file occurrences. Combined MSX/MSX2/MSX2+/Turbo R source folders remain in a distinct `msx-family` category, without guessed game compatibility. Both the exact-blob importer and full index build passed CI; **1,125 previously absent cheat-file occurrences** and two original notices are now archived, producing a new MSX-family index bundle.

**Latest external source expansion:** Nine native platform/format partitions now expose **691 additional pinned original file references** from Eignar17's FinalBurn Alpha/legacy DAT arcade sources, Slade1972's Sega CD MiSTer ZIPs, SkillerCMP's original Xbox Xemu text cheats, and Riyoway's Nintendo database exports. These 691 file occurrences correspond to **487 distinct Git blobs**, all absent from the pre-onboarding archive and earlier external source catalogs. They are **metadata-only links**, not imported cheat code bodies or verified games. Examples: `cheatarium-query external --db generated/v1 --platform segacd --title "3 Ninjas" --json` and `--platform xbox --title EA0049`.

**New Nintendo GameCube/Wii U bulk source leads:** Three pinned GameCube WiiRD regional ZIP archives (NTSC-J, NTSC-U, PAL) and 13 original Wii U XML/JSON documents from two community collections are now searchable in the offline **external** catalog. The 16 original file references occupy **15 distinct Git blobs** (two Wii U JSON source files are identical). The GameCube ZIPs contain unknown internal cheat-file counts until independently inspected. All remain rights-unresolved, link-only, and untested. The external catalog now contains **43,454** file references across **51** source catalogs.

## Status

The full MIT-noticed, Artemis-derived PS3 **source/compiled** snapshot is now archived and searchable: **4,343** compiled cheat-source files, **13,343** editable sources/aliases and **2** notices (**17,688** new original file paths). The compiled files add **4,343 searchable source records** and **86,831 original code-bearing index entries**, but the collection reformats already archived Artemis material, so these are **not 4,343 independently discovered games or 86,831 newly discovered cheat effects**. Individual cheat authorship and gameplay compatibility remain unverified.


Cheatarium preserves **67,834 archived original files, including cheat data, supporting source files and notices, from 81 pinned upstream collections**. Its complete pinned Libretro snapshot covers all **51 source directories** (27,915 original `.cht` files); the latest console-focused bulk imports add **75 PlayStation 5-native etaHEN cheat files, 58 PS Vita rinCheat originals, the entire 6,375,204-byte PS2 CheatDevice database, 7,244 original PS2 ZIP members, 473 CRC-labeled PS2 PNACH files and 2,992 Nintendo Switch NXCheats native cheat text files**. The **PS2 CheatDevice database** contains **1,483 source-declared game sections** and **200,941 original hexadecimal code lines**; those figures do not describe the Switch collection. Original bytes, pinned Git blob IDs, source notices and contributor rights caveats are preserved.

**Searchable:** **57,351 indexed source/game-section records across 59 platform bundles**, with **47,502 source-derived candidate game groups**, **1,825,471 code-bearing index entries**, and **21,322 native memory address/value entries**. **These are NOT 57,351 physically distinct source files:** a single original PS2 CheatDevice text file produces 1,483 separately searchable source-game records. Exactly 34 archived Dolphin INIs remain unassigned to a console family. The 67,831-file archive count includes notices and supporting originals, and the 620 Nintendo Switch source-derived candidate groups are not confirmed distinct games. PS2 raw device lines, code-group entries and source game sections are different units and must not be conflated. No source title is proof of verified game/revision identity, and no newly imported cheat is claimed gameplay tested.

The platform registry covers **67 systems/environments** (including classic consoles, handhelds, arcade, home computers and game engines); three games have manual curated records. SNES research decodes over 66,000 historical fields; see the [SNES audit](generated/v1/reports/snes-codec-coverage.json). **Next priority remains complete bulk console acquisition and measurable coverage expansion, not isolated game-by-game curation.** Earlier source authors' rights cannot automatically be inferred from host repository licenses.

**Switch filename/build-ID coverage:** The [strict cross-source audit](generated/v1/reports/switch-external-title-build-coverage.json) compares native 16-hex Switch title/build-ID file labels across imported Sharkive/NXCheats and the blocked Ademola/ibnux collections, keeping title-only JSON mirror labels and graphics patches separate. It does not infer game identity, code effects, or compatibility.

**3DS native title-ID coverage:** A [new source-label report](generated/v1/reports/3ds-external-title-id-coverage.json) compares archived Sharkive title-ID filenames with metadata-only Ademola and JourneyOver native 3DS cheat filenames. Unstructured ASM documents remain outside ID overlap claims; these are not verified games, tested effects, or reproduction rights.

**PS2 filename/CRC coverage:** The [strict external PNACH filename audit](generated/v1/reports/external-ps2-crc-labels.json) compares source-declared CRC labels and exact serial+CRC tokens across the four pinned PS2 catalogues, without loading cheat code bodies or inferring game compatibility. This makes cross-source bulk acquisition overlap measurable independently of byte-level duplicates.

The archive is separate from any particular emulator, but future integrations may make these cheats directly usable in projects such as Starbyte.


## Public-domain built-in homebrew secrets

A separately preserved [retro homebrew/indie secrets collection](https://github.com/bbbbbr/retro-homebrew-cheat-codes) supplies **19 source-declared entries from one original JavaScript data file**, including button combinations, menu tricks and unlocks. Its unchanged README and Unlicense are archived alongside the original data. These are **not emulator memory-write cheat entries** and do not change the 1.8-million code-field count. A provenance-checked read-only index lives at `generated/v1/secrets/homebrew.json`. Query it with `python3 tools/build_homebrew_secrets.py --search 'Canyon Racer' --platform gb`. Game effects are source claims, not independently tested.


**PS1 derivative preservation gaps:** The pinned [crashGG DuckStation fork](https://github.com/crashGG/ps1.chtdb) contains 4,485 PS1 cheat/patch originals; 4,217 match bytes already in the DuckStation link catalog. Cheatarium stores only its **268 byte-different filename references** (263 cheats and 5 patches) in the searchable external index, without copying their codes. Different bytes do not prove distinct games, effects, or original-author rights. The fork's LICENSE explicitly reserves cheat/patch copyrights to their creators.


**Two further compiled handheld collections are pinned link-only:** [DeadSkullzJr's Nintendo DS database format mirror](https://github.com/szTheory/NDS-Cheat-Databases) contributes nine original compiled source-file occurrences across eight distinct Git blobs (including a 103 MB XML output and two byte-identical .db files). [Saramagrean's PSP CWCheat database](https://github.com/Saramagrean/CWCheat-Database-Plus-) contributes one 8.9 MB `cheat.db` original. These are alternative compiled archives, not ten verified cheats or independently distinct game datasets. Their upstream notices cite other original authors, and redistribution rights are unresolved; only source-file metadata and pinned URLs are indexed.


**MAME arcade archive sources:** Pinned link-only references now include the upstream Libretro **MAME 2010** `metadata/cheat.zip` (24,981,772 bytes) and **MAME 2003-Plus** `metadata/cheat.dat` (7,810,253 bytes). The MAME 2010 ZIP is byte-identical to `galisteogames/mame2010_cheat`'s mirror: it is counted once. Their original authors' cheat-body reuse terms remain unresolved. MAME is tracked under `arcade-mame`, *not* FinalBurn Neo. These two archive files have no extracted per-game counts.

**PS2 derivative PNACH source:** `shadowninja826/pcsx2_pnach_cheats` offers 628 files, but 579 share exact bytes with pinned `xs1l3n7x-pcsx2` and one additional file is empty. Only its 48 byte-distinct, nonempty PNACH file links are added; neither game uniqueness nor functioning codes are claimed.


**More legacy retrocheat catalogs (links, not bodies):** Smoker1's pinned FinalBurn Alpha "Official Cheats" folder contains **98 .ini file occurrences representing just 45 distinct Git blobs**; 53 repeated-file occurrences are exact byte aliases for clone variants. They are labeled arcade-fba, not MAME or FinalBurn Neo, with no gameplay or version compatibility asserted. The pinned pacnpal/mupen-cheats repository adds one **1,281,051-byte native N64 Mupen64Plus mupencheat.txt** reference. The author describes changes to GoldenEye, Perfect Dark and Donkey Kong 64 codes mixed into an older base. Both remain link-only pending original-author permission; neither compiled text source is counted as an independently verified number of titles or effects.


**Historical RetroArch Git-blob delta now searchable:** Two pinned older forks contributed only 86 .cht path occurrences outside exact bytes already compared from Libretro, Emupedia and BitHeX, despite containing 11,390 original .cht files combined. The 86 paths contain 77 byte-distinct Git blobs, not necessarily 77 new games or 77 usable cheats. They are grouped into seven rights-unresolved external catalogs for NES, GBA, SNES, FBNeo, Dreamcast, NAOMI and miscellaneous RetroArch cores. The immutable [three-corpus mirror audit](research/retroarch-mirror-source-gaps-v1.json) supplies the exact path/hash evidence. These source-family gaps may overlap other archived collections; all remain **link-only**.
