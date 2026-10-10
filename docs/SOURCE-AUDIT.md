# Further archive acquisition: source rights and format audit

Cheatarium has exhausted its **39 mapped Libretro console/handheld directories** for the pinned revision. Other consoles and additional codes require independent, source-specific ingestion. Public GitHub visibility by itself is not permission to redistribute third-party collections.

| Source | Platforms | Native format | Preliminary status |
| --- | --- | --- | --- |
| [FlagBrew Sharkive](https://github.com/FlagBrew/Sharkive) | 3DS / Switch | 3DS title-ID Gateshark and Switch title/build-ID Atmosphère `.txt` | **Imported**: 1,135 original cheat text files at pinned `aeab5fd3`; GPLv3 is declared for project; original user contributions require preserved attribution and may involve separate third-party rights. Full native paths, blobs, LICENSE, README retained. |
| [AeroWidescreen PCSX2 Cheats](https://github.com/AeroWidescreen/PCSX2-Cheats) | PS2 | 62 native PNACH patches including controls, widescreen and gameplay edits | **Imported** at pinned `ab043ba5`. MIT root LICENSE, README, native regional and CRC filenames and original author text preserved. |
| [Dolphin GameSettings](https://github.com/dolphin-emu/dolphin) | GameCube / Wii / unresolved | 278 native INI files containing explicit ActionReplay or Gecko named code payloads | **Imported** at pinned `e6f3ae17`. 210 classified GC, 34 Wii, 34 unresolved. Ordinary emulator preference files omitted; original COPYING, Readme, four license notices retained. Dolphin describes GPLv2+ for most original work; individual derived/third-party terms may vary. |
| [CookiePLMonster Console Cheat Codes](https://github.com/CookiePLMonster/Console-Cheat-Codes) | PS2 / PS1 / PSP / GameCube | 110 PNACH, 67 native PS1 CHT, 4 PSP CWCheat INI, 2 GameCube INI | **Imported**: 183 exact source cheat files, 204 original repository files including metadata. Pinned `f55aa8db`; root MIT license and explicit per-author credits retained. Distinguish upstream original/adapted codes and unverified compatibility. |
| [PCSX2 cheats collection](https://github.com/xs1l3n7x/pcsx2_cheats_collection) | PS2 | `.pnach`; game serial/CRC | Link only. No root license found; README acknowledges multiple third-party source dumps. |
| [Dolphin Gecko examples](https://github.com/hakami55/Gecko-Codes-for-Dolphin) | GameCube, Wii | Gecko blocks with game ID/revision | Link only. README itself contains codes but no root license found. |
| [Switch Cheats DB](https://github.com/HamletDuFromage/switch-cheats-db) | Switch | Atmosphère title/build IDs | Link only. Mirrors GBATemp and other community authors; redistribution terms not established. |
| [GoldHEN Cheat Repository](https://github.com/GoldHEN/GoldHEN_Cheat_Repository) | PS4-oriented | JSON / MC4 / SHN / XML | **Imported 5,445 originals** at pinned `acc22fef`; upstream GPLv3 license, initial PS4Trainer and individual author credits retained; per-contributor rights remain an open provenance caveat. |
| [Artemis PS3](https://github.com/bucanero/ArtemisPS3) | PS3 | Native `.ncl` | **Imported 2,540 originals** at pinned `69b1c537`; MIT repository license and original README/inline cheat-author credits preserved; third-party rights not independently determined. |
| [GameHacking.org](https://gamehacking.org/) | Many consoles | Various device codes | Discovery/link-only pending permission and author attribution review. |
| [Admentus Enhancement Codes](https://github.com/Admentus64/Enhancement-Codes) | GameCube / Wii VC | Dolphin `.ini`, Action Replay / Gecko | **Imported 21 originals** at pinned `f8e9ad12`; upstream GPLv3 and Credits.txt archived; original individual AR/Gecko authors explicitly credited. |
| [mkwcat Gecko Codes](https://github.com/mkwcat/gecko-codes) | Wii / Wii U | Gecko code Markdown with region/revision variants | **Imported 28 originals** at pinned `5b31c6c1`; author explicitly declares MIT with attribution request. |

Imported originals are separately inventoried in per-repository `sources/*.json` manifests, with native paths, pinned Git revisions, Git blob checksums and archived attribution/license notices. Upstream projects may contain historical third-party contributions under different terms; preservation of a project-wide GPL or MIT notice is **not** an assertion that every original cheat author supplied blanket relicensing permission. Rights-unresolved source candidates remain link-only in `sources/candidates.json`.

## Next extraction methods

- PS2: preserve `.pnach` filenames, CRC keys, serial/build identifiers and original patch type/value. Do not interpret a CRC as a universal game ID.
- GameCube/Wii: preserve Game ID, country/region, revision, engine variant and multiline Gecko payloads. Never assume Gecko and raw memory writes are interchangeable.
- Switch: preserve title ID, build ID, mod type, code provenance and version dependency independently.
- PS3/PS4: keep original package/data formats and required licenses alongside imported data; do not conflate homebrew launcher code with cheat payload licensing.
- Multiple sources of the same code: keep occurrence identity and all source credits before any normalized cross-source grouping.

Until rights are resolved, use link-only registry entries or privately processed analysis without publishing a copied dataset. Do not mislabel imported data as verified solely because an upstream repo describes a successful test.

Reviewed 2026-10-09. Repository structure, source-level import integrity and published upstream licensing, not individual cheat functionality or full per-contributor licensing, were inspected.

**Collection accounting (2026-10-10):** **33,091 original cheat files archived** from ten pinned source repositories; **33,057 indexed original files in 44 console bundles**, with 34 Dolphin INIs archived but intentionally left without platform assignment. Of the new Dolphin code-bearing source files, 210 are indexed as GameCube and 34 as Wii. 62 more independently authored PS2 PNACH files were also indexed. The 1,257,967 indexed code-bearing source occurrences are not asserted to be unique, compatible, or empirically verified cheats. GoldHEN PS2-era identifiers remain PS4-oriented source metadata and do not supply native PS2 cheat support.
