# Cheatarium

**A wonderland of console cheats.**

Cheatarium collects, preserves, organizes, and explains video game cheats: Game Genie and Action Replay codes, GameShark and other device codes, emulator cheats, button sequences, passwords, unlockables, and strange ways to bend a game to your will.

This is primarily a **console and handheld game** archive. The goal is a fun, searchable collection that respects the original community work and remains useful decades later.

## Explore

- [Curated games](curated/) — human-friendly cheats grouped by platform and game, including versions and code formats.
- [Original archives](archive/) — unmodified, attributable imports in their original formats.
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

- [Pinned external source metadata](sources/external-bulk/) — filename, blob hash and byte-count indexes for 4,497 DuckStation PS1 cheats, 138 PS1 patches, 501 Xenia Xbox 360 patches, 4,710 official PCSX2 PS2 patches, 796 VitaCheat sources from two databases and 387 Dreamcast source documents. These **11,029 upstream files are not archived**: rights are unresolved and some patches are not cheats. Source links are read-only references, not independently tested compatibility evidence. Use `python3 tools/external_bulk.py --search "SLUS-" --platform ps1` for offline filename lookup.

## Status

The full MIT-noticed, Artemis-derived PS3 **source/compiled** snapshot is now archived and searchable: **4,343** compiled cheat-source files, **13,343** editable sources/aliases and **2** notices (**17,688** new original file paths). The compiled files add **4,343 searchable source records** and **86,831 original code-bearing index entries**, but the collection reformats already archived Artemis material, so these are **not 4,343 independently discovered games or 86,831 newly discovered cheat effects**. Individual cheat authorship and gameplay compatibility remain unverified.


Cheatarium preserves **66,704 archived original files, including cheat data, supporting source files and notices, from 80 pinned upstream collections**. Its complete pinned Libretro snapshot covers all **51 source directories** (27,915 original `.cht` files); the latest console-focused bulk imports add **75 PlayStation 5-native etaHEN cheat files, 58 PS Vita rinCheat originals, the entire 6,375,204-byte PS2 CheatDevice database, 7,244 original PS2 ZIP members, 473 CRC-labeled PS2 PNACH files and 2,992 Nintendo Switch NXCheats native cheat text files**. The latter contains **1,483 source-declared game sections** and **200,941 original hexadecimal code lines**. Original bytes, pinned Git blob IDs, source notices and contributor rights caveats are preserved.

**Searchable:** **56,226 indexed source/game-section records across 58 platform bundles**, with **46,802 source-derived candidate game groups**, **1,819,305 code-bearing index entries**, and **21,322 native memory address/value entries**. **These are NOT 56,226 physically distinct source files:** a single original PS2 CheatDevice text file produces 1,483 separately searchable source-game records. Exactly 34 archived Dolphin INIs remain unassigned to a console family. The 66,704-file archive count includes notices and supporting originals, and the 620 Nintendo Switch source-derived candidate groups are not confirmed distinct games. PS2 raw device lines, code-group entries and source game sections are different units and must not be conflated. No source title is proof of verified game/revision identity, and no newly imported cheat is claimed gameplay tested.

The platform registry covers **67 systems/environments** (including classic consoles, handhelds, arcade, home computers and game engines); three games have manual curated records. SNES research decodes over 66,000 historical fields; see the [SNES audit](generated/v1/reports/snes-codec-coverage.json). **Next priority remains complete bulk console acquisition and measurable coverage expansion, not isolated game-by-game curation.** Earlier source authors' rights cannot automatically be inferred from host repository licenses.

The archive is separate from any particular emulator, but future integrations may make these cheats directly usable in projects such as Starbyte.
