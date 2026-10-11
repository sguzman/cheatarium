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

Cheatarium preserves **33,267 original cheat-bearing source files from 70 pinned upstream collections**, keeping source-native paths, upstream Git blob hashes, original licensing, author attribution and rights caveats. The newest two collections contribute **one PS3** PowerPC patch Markdown original for Need for Speed Carbon and **one Nintendo 3DS** Ice Station Z first-person CTRPF original. The PS3 source includes separately labeled unlock, debug, rendering and MSAA alternatives, with its original auxiliary patch-input files; the 3DS source retains Foxy#XTN authorship and Peanut#XTN idea attribution. Previous acquisitions included Wii disc and WiiWare Gecko codes, PS2 Ratchet & Clank PNACHs, PSP original cheats and GameCube code sources.

**Searchable:** **33,233 original files in 44 console index bundles**, including **236 PS2**, **2,660 PSP**, **233 GameCube**, **168 Wii**, **2,541 PS3**, and **660 Nintendo 3DS** native files. **34 original Dolphin INIs** remain archived but unassigned to a console family. The reproducible catalog records **1,259,035 code-bearing source entries** and **13,296 native memory address/value entries**. Code counts and source game/build hints are not proof of distinct working cheats or tested execution. Host repository licenses do not automatically clear earlier independent cheat authors' rights.

All source IDs, release/region labels and emulator build identifiers are source-derived suggestions, not independently verified game/ROM matches. Third-party rights and original author credits remain separate from the MIT license covering Cheatarium's original software. The platform registry currently covers 55 systems; three games have manual curated records. SNES research decodes more than 66,000 historical fields, with exact coverage in the [SNES audit](generated/v1/reports/snes-codec-coverage.json). More primary source acquisition, title/build verification and actual cheat testing remain future work.

The archive is separate from any particular emulator, but future integrations may make these cheats directly usable in projects such as Starbyte.
