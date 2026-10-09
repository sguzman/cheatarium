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

Cheatarium now preserves **32,551 original cheat files** across six attributable upstream repositories: 23,382 Libretro `.cht` files, 1,135 Sharkive 3DS/Switch text files, 5,445 GoldHEN PS4-oriented JSON/MC4/SHN/XML files, 2,540 Artemis PS3 `.ncl` files, and 49 GameCube/Wii/Wii U Action Replay/Gecko-code files from two original-author collections. Each upstream is pinned to a Git revision with original paths, Git blob hashes, and archived license/credit notices. GoldHEN source IDs that refer to PS2-era games do **not** establish native PS2 cheat compatibility. Third-party author rights remain distinct from the repository-level licenses.

**Searchable today:** 24,517 original files in **38 console index bundles** (Libretro plus Sharkive); the newly preserved PS3/PS4/Gecko archives are not yet indexed. Those 38 bundles contain **1,192,989 source code-bearing entries** and **13,296 native memory address/value entries**. None of these counts proves unique gameplay effects or executable compatibility. The platform registry covers 55 systems, and three games have manual curated records. The SNES source index interprets **over 66,000 code fields** from source-declared device formats or recognizable unlabeled syntax, while excluding independently documented revision alternatives from combined-write output. The exact, reproducible counts live in the [SNES coverage audit](generated/v1/reports/snes-codec-coverage.json); original cheat strings remain untouched. Further source collections, reliable ROM/build matching, and actual cheat execution remain future work.

The archive is separate from any particular emulator, but future integrations may make these cheats directly usable in projects such as Starbyte.
