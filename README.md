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

## Status

Cheatarium preserves **33,146 original cheat-bearing source files from 33 pinned upstream collections**, retaining native paths, original Git blob hashes, author credits, rights notices, and per-source licensing. The latest acquisition sequence delivered **24 additional native console cheat files from 13 pinned repositories**: **14 Wii Gecko INIs** (Sonic Riders: Zero Gravity, SpongeBob: Creature from the Krusty Krab, Okami, Meet the Robinsons and Punch-Out!!), **8 Nintendo 3DS CTRPF/Gateshark text files** (seven separately licensed FoxyXTN projects), and **2 PlayStation 2 PNACHs** (Need for Speed Underground). Earlier additions include AeroWidescreen (62 PS2), Dolphin (278 GameCube/Wii), SalustLab/PreachingPython (4 PS2), igorciz777/Saupernova13 (15 PS2), and the founding Libretro, Sharkive, GoldHEN, Artemis PS3, Gecko, CookiePLMonster and Igor archives.

**Searchable:** **33,112 original cheat files across 44 console index bundles**, including **215 PS2**, **218 GameCube**, **90 Wii** and **656 3DS**. **34 original Dolphin INIs remain archived with unresolved console-family assignment** and are not counted as searchable console records. The reproducible catalog exposes **1,258,506 code-bearing source entries** and **13,296 native memory address/value entries**. Code-bearing occurrences are not necessarily distinct verified cheats or gameplay effects; metadata-only and opaque records are labeled separately.

All source IDs, release/region labels and emulator build identifiers are source-derived suggestions, not independently verified game/ROM matches. Third-party rights and original author credits remain separate from the MIT license covering Cheatarium's original software. The platform registry currently covers 55 systems; three games have manual curated records. SNES research decodes more than 66,000 historical fields, with exact coverage in the [SNES audit](generated/v1/reports/snes-codec-coverage.json). More primary source acquisition, title/build verification and actual cheat testing remain future work.

The archive is separate from any particular emulator, but future integrations may make these cheats directly usable in projects such as Starbyte.
