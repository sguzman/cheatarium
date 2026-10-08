# Further archive acquisition: source rights and format audit

Cheatarium has exhausted its **39 mapped Libretro console/handheld directories** for the pinned revision. Other consoles and additional codes require independent, source-specific ingestion. Public GitHub visibility by itself is not permission to redistribute third-party collections.

| Source | Platforms | Native format | Preliminary status |
| --- | --- | --- | --- |
| [PCSX2 cheats collection](https://github.com/xs1l3n7x/pcsx2_cheats_collection) | PS2 | `.pnach`; game serial/CRC | Link only. No root license found; README acknowledges multiple third-party source dumps. |
| [Dolphin Gecko examples](https://github.com/hakami55/Gecko-Codes-for-Dolphin) | GameCube, Wii | Gecko blocks with game ID/revision | Link only. README itself contains codes but no root license found. |
| [Switch Cheats DB](https://github.com/HamletDuFromage/switch-cheats-db) | Switch | Atmosphère title/build IDs | Link only. Mirrors GBATemp and other community authors; redistribution terms not established. |
| [GoldHEN Cheat Repository](https://github.com/GoldHEN/GoldHEN_Cheat_Repository) | PS4 | JSON, MC4, SHN | Root GPL-3 license observed; review individual content/attribution and license separation. |
| [Artemis PS3](https://github.com/bucanero/ArtemisPS3) | PS3 | Artemis community codes | Root MIT license observed for software; verify contributed cheat-code rights separately. |
| [GameHacking.org](https://gamehacking.org/) | Many consoles | Various device codes | Discovery/link-only pending permission and author attribution review. |

The machine-readable queue is `sources/candidates.json`; entries represent candidates, **not imported content**. Every actual import must receive its own immutable `sources/<source>.json` manifest, pinned revision, unchanged archive files, and appropriate license notices.

## Next extraction methods

- PS2: preserve `.pnach` filenames, CRC keys, serial/build identifiers and original patch type/value. Do not interpret a CRC as a universal game ID.
- GameCube/Wii: preserve Game ID, country/region, revision, engine variant and multiline Gecko payloads. Never assume Gecko and raw memory writes are interchangeable.
- Switch: preserve title ID, build ID, mod type, code provenance and version dependency independently.
- PS3/PS4: keep original package/data formats and required licenses alongside imported data; do not conflate homebrew launcher code with cheat payload licensing.
- Multiple sources of the same code: keep occurrence identity and all source credits before any normalized cross-source grouping.

Until rights are resolved, use link-only registry entries or privately processed analysis without publishing a copied dataset. Do not mislabel imported data as verified solely because an upstream repo describes a successful test.

Reviewed 2026-10-08. Repository structure and published licensing, not individual cheat functionality, were inspected.
