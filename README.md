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

Cheatarium preserves **37,805 original cheat-bearing source files from 73 pinned upstream collections**. The latest **bulk-first** acquisition pass completed all **51 pinned Libretro cheat directories**: **27,915 source-native `.cht` files**, including 4,533 newly imported files from MSX, MSX2, FinalBurn Neo arcade, DOS, Atari 8-bit, ZX Spectrum +3, and six further source environments. This does **not** mean every console or game is covered; these additional platforms include home computers, arcade and game engines. Source-native paths, original Git blob IDs, repository licensing, author credits and rights caveats remain preserved.

**Searchable:** **37,771 original files across 56 platform index bundles**, comprising **31,210 source-derived candidate game groups**, **1,282,537 code-bearing entries**, and **21,322 native memory address/value entries**. These figures are not verified distinct games, tested cheats, or guaranteed emulator compatibility. **34 original Dolphin INIs** remain preserved but not assigned to a console family. Further bulk console acquisition is the next priority, not manually selecting isolated titles.

Original releases, game/ROM identity, and emulator compatibility are unverified except for explicitly evidenced records; historical source-license labels do not automatically settle earlier cheat authors' rights. The platform registry contains **67 system/environment identifiers**, and three games have manual curated records. SNES research decodes more than 66,000 historical fields; see the [SNES audit](generated/v1/reports/snes-codec-coverage.json).

The archive is separate from any particular emulator, but future integrations may make these cheats directly usable in projects such as Starbyte.
