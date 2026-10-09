# Upstream source registry

Redistribution status matters as much as code count. Register candidates here; only copy their payloads after checking permissions and notices.

| Source | Scope | Status | Notes |
| --- | --- | --- | --- |
| [Libretro Database](https://github.com/libretro/libretro-database) | Many retro consoles/handhelds; RetroArch `.cht` | **39 source collections imported** | Repository CC BY-SA 4.0; 23,382 archived `.cht` files; [pinned provenance and file inventory](libretro-database.json). Check original credits. |
| [FlagBrew Sharkive](https://github.com/FlagBrew/Sharkive) | Nintendo 3DS Gateshark and Switch Atmosphère text | **1,135 original cheat files imported** | Pinned upstream commit, native title/build-ID paths and Git blobs preserved in [source inventory](sharkive.json). GPL-3.0 declared upstream; original contributors' attribution/rights must still be respected. LICENSE and original README retained. |
| [GameHacking.org](https://gamehacking.org/) | Extremely broad code collection and research | **Link / rights review** | Great research and export source. Do not bulk rehost without confirming permissions. |
| [Switch Cheats DB](https://github.com/HamletDuFromage/switch-cheats-db) | Switch title/build-ID cheats | **Candidate / rights review** | Mirror of other community sources; title/build IDs critical. No redistribution clearance established here. |
| [PCSX2 Cheats Collection](https://github.com/xs1l3n7x/pcsx2_cheats_collection) | PlayStation 2 `.pnach` files | **Candidate / rights review** | Large CRC-keyed collection; confirm contributor/reuse terms before importing. |
| [Gecko Codes for Dolphin](https://github.com/hakami55/Gecko-Codes-for-Dolphin) | GameCube/Wii Gecko codes | **Candidate / rights review** | Game IDs, region and revision specificity; retain author credit and check redistribution rights. |
| [GoldHEN Cheat Repository](https://github.com/GoldHEN/GoldHEN_Cheat_Repository) | PlayStation 4 | **Candidate / review** | GPL-3.0 repository license observed; distinguish tools/data and their notices before copying. |
| [Artemis PS3](https://github.com/bucanero/ArtemisPS3) | PlayStation 3 | **Candidate / review** | Project points to community cheat codes and an online database; check individual code rights and format. |

The [machine-readable acquisition queue](candidates.json) and [source/rights audit](../docs/SOURCE-AUDIT.md) track additional candidates without copying their payloads.

The **source manifest**, not this table, is the authoritative record of what has actually been imported. Dates, revisions, and license details must not be inferred for sources listed as candidates.

### Attribution

The pilot import uses `libretro/libretro-database` at commit `fbeefcb46c2e1b20a7e2945f34a694a41b2d6f90`, accessed 2026-10-08. Credits: Libretro Database contributors and upstream original authors. The upstream repository lists CC BY-SA 4.0. Each intact `.cht` file retains its original title and data. Attribution and any changes in future curated derivatives must be preserved. A repository-level license does not automatically settle every third-party contribution.

### Sharkive acquisition (2026-10-09)

The first pinned import contains **644 3DS text files** and **491 Switch text files** from upstream commit `aeab5fd3b001ed22c013efd1a02b092575d825ed`. The source manifest records original paths and exact Git blob hashes; original GPLv3 LICENSE and credit-bearing README are archived alongside the native text files. Three Switch build-ID filenames are nonstandard lengths and are preserved unchanged rather than renamed or interpreted as verified builds. There are 18,417 original bracketed section headings; these are archival text markers, not unique or tested effects. The generated distribution now includes **38 platform search bundles**: 36 original Libretro bundles plus 3DS and Switch. Sharkive native cheat sections are searchable by original title/build-ID paths with untouched multiline payloads, source ordinals and checksum-checked provenance. The file paths do not supply independently verified game names or ROM/build compatibility. Source-level reuse claims remain bounded by actual upstream licensing and contributions.
