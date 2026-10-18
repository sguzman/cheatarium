# Cheatarium consumers

Cheatarium publishes **read-only**, revision-pinned, compressed JSON and a Rust client in `crates/cheatarium-client`. It is not an emulator and does not modify emulator repositories.

The 58-platform bulk distribution includes PS5-native etaHEN, original PS Vita rinCheat runtime text/save-editor Lua, and complete source-declared PS2 CheatDevice game sections. PS2's 1,483 separately searchable records come from **one original 6.4 MB source file** and retain the same original provenance hash; per-record source IDs include exact original game-section ordinals. The PS2 parser groups contiguous native hexadecimal device-code lines under source labels; code-group counts are not raw line counts, distinct validated effects, or evidence that a code executes. Vita Lua is indexed as inert original source data, never executed. Each imported record remains unverified, and actual game-build compatibility must be established separately.

## SNES files

- `generated/v1/catalog.json`: the machine-readable console catalog.
- `generated/v1/games/snes.json.gz`: advisory title groups linking to their source records.
- `generated/v1/repeats/snes.json.gz`: conservative exact-text repetitions linking back to distinct source files.
- `generated/v1/tags/snes.json.gz`: English lexical signals from source cheat descriptions.
- `generated/v1/taxonomy/effects-v1.json`: versioned, inspectable phrases driving those tags.
- `generated/v1/reviews.json`: independently evidenced gameplay-effect review register (five external reports, zero tested observations).
- `generated/v1/snes.json.gz`: full parsed source occurrences, original code strings, hints and provenance.
- `generated/v1/identities/snes.json`: separately reviewed ROM hash evidence registry (currently empty; no guessed ROM mappings).
- `generated/v1/interpretations/snes.json`: evidence-backed revision alternatives for exact original multi-part source codes; all other SNES `+` joins remain unresolved.
- `generated/v1/interpretations/snes-published-groups.json`: historical source-publication witnesses, linked to exact original source ordinals and Git blobs; never evidence of successful execution.
- `generated/v1/reports/snes-composition-review-queue.json`: optional, ranked unresolved source-join review priorities with complete source identifiers and advisory candidate names.
- `generated/v1/distribution.json`: SHA-256 hashes and byte sizes of all artifacts.

An emulator can download the relevant platform artifacts at a **pinned Cheatarium commit**; it does not need the raw archives. Filenames and title-group keys are suggestions, never trusted release/ROM identities.

## External PS2 filename/CRC overlap

`generated/v1/reports/external-ps2-crc-labels.json` measures strict native `.pnach` filename CRC and serial-label overlap across the pinned official PCSX2, consolidated DAOWAce and xs1l3n7x repositories. It accepts only complete eight-digit hexadecimal CRC labels (alone, as a leading code or after a serial) and explicitly leaves malformed names unresolved. CRCs are **source filename tokens, not authenticated ROM hashes**. No cheat payloads or author permissions are inferred; the report is included in the checksum-verified distribution manifest.

## External pinned source discovery (metadata only)

Cheatarium provides **43,454 rights-unresolved source-file references in 51 pinned source catalogs across 26 external search platform labels** under checksum-checked `generated/v1/external/catalog.json` and platform bundles. The additional 691 pinned arcade, Sega CD, original Xbox and Nintendo export references are source paths and blob hashes only (487 byte-distinct Git blobs absent from the pre-onboarding archive and external inventories), not playable cheat bodies. Only source IDs, file paths, original Git blob identities, byte sizes, pinned URLs and source-native cheat/patch labels are exported. They do not confer code-author reuse rights, game identity, execution support or tested compatibility.

```sh
cargo run --release -p cheatarium-client --bin cheatarium-query -- external --db generated/v1 --platform xbox-360 --title "Painkiller" --limit 10 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- external --db generated/v1 --platform vita --title PCSA --source-id r0ah-vitacheat --limit 10 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- external --db generated/v1 --title SLUS --offset 0 --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- external --db generated/v1 --platform ps3 --title USERLIST --limit 10 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- external --db generated/v1 --platform psp --title cheat.db --json
```

The Rust client checks the exported external catalog and selected platform bundles against `distribution.json` **before** searching. That confirms integrity relative to the pinned manifest, not that the manifest is authenticated or the codes work. Import permissions remain unresolved, so external references must never be silently promoted into source cheat records, a known-game identity or an activatable code.

## Rust client

The client exposes `load_catalog`, `load_game_candidates`, `load_platform`, `verify_platform_distribution`, `publications::load_snes_publications`, and candidate title searches. Its CLI supports these local-only operations:

```sh
cargo run --release -p cheatarium-client --bin cheatarium-query -- platforms --db generated/v1 --offset 0 --limit 44 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- discover --db generated/v1 --title 'Donkey Kong Country' --offset 0 --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- game --db generated/v1 --platform 3ds --game-key title-id-0004000000030c00 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- entries --db generated/v1 --platform switch --game-key title-id-0100000000010000-build-b424be150a8e7d78 --source-id sharkive --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- games --db generated/v1 --platform snes --title 'Chrono Trigger' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- game --db generated/v1 --platform snes --game-key push-over --offset 0 --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- entries --db generated/v1 --platform snes --game-key push-over --role code --offset 0 --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- entry --db generated/v1 --platform snes --source-record-id 'libretro-database:cht/Nintendo - Super Nintendo Entertainment System/Push-Over (USA).cht' --ordinal 3 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- entries --db generated/v1 --platform snes --game-key push-over --role code --description-contains 'lives' --offset 0 --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- entries --db generated/v1 --platform snes --game-key push-over --role code --region-hint USA --source-id libretro-database --description-contains 'level' --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- search --db generated/v1 --platform snes --title 'Chrono Trigger' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- source --db generated/v1 --platform snes --source-record-id 'libretro-database:cht/Nintendo - Super Nintendo Entertainment System/Push-Over (USA).cht' --offset 0 --limit 20 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- effects --db generated/v1 --platform snes --effect 'Infinite Lives' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- codes --db generated/v1 --platform snes --code '6D6B-6F0F' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- repeats --db generated/v1 --platform snes --game-key super-mario-world --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- compositions --db generated/v1 --platform snes --game-key donkey-kong-country --relation revision-alternatives --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- reviews --db generated/v1 --platform snes --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- publications --db generated/v1 --platform snes --game-key push-over --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- reviews --db generated/v1 --platform snes --category lives --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- tags --db generated/v1 --platform snes --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- tags --db generated/v1 --platform snes --category lives --limit 10 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- effects --db generated/v1 --platform snes --effect 'Infinite' --title 'Mario' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- effects --db generated/v1 --platform snes --effect 'Infinite' --declared-format game-genie --source-id libretro-database --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- verify --db generated/v1 --platform snes --json
```

`compositions` shows the exact source `+` joins, whether a specific original code is independently documented as a revision-alternative partition, and the source-cited grouping. Optional `--source-record-id` and `--game-key` filters support research scoped to one original source or one advisory candidate. `--offset` and `--limit` page source occurrences while keeping unfiltered totals in the result. An unknown join does not become a simultaneously executable program. Even a reviewed version alternative is not tied to an exact ROM revision: no code selection or execution happens. See [SNES code composition](SNES-CODES.md).

`publications` returns the separately validated, source-bound SNES publication-witness registry. It supports `--game-key`, `--source-record-id`, `--offset`, and `--limit`; JSON reports total, returned, and remaining-page status. The Rust client independently checks each witness against its original code string, source ordinal and Git blob. These are *published code text* matches, not gameplay observations or compatibility assertions. No code is executed or activated.

`reviews` lists separately sourced reports and exact-ROM test observations, optionally narrowed by original `--source-record-id` or `--category`. It currently returns five externally reported claims (three SNES and two NES) and zero tested observations. No text-only match has been promoted to an empirically verified cheat. See [effect reviews](EFFECT-REVIEWS.md).

`tags` lists English-language cue counts or, with `--category`, source-record occurrences and the exact phrase that matched. Optional `--game-key` narrows category results to a **filename-derived candidate**. A textual phrase can be negated, misleading or mistranslated and does not establish functionality. See [Effect signals](EFFECT-SIGNALS.md).

The `games` title-group search and `search` original-source title search now both accept `--offset` and `--limit`. Their JSON includes the unpaginated result total, page offset, actual returned count and `has_more`, so broad terms can be traversed without silently dropping later matches. Search results remain filename candidates, not verified game identities.

**PS3 / PS4 / GameCube / Wii / Wii U original archives:** Pinned Artemis NCL, GoldHEN JSON/SHN/MC4/XML and independent Gecko/Action Replay records are now indexed. Entries retain upstream descriptions, original author fields where supplied, exact source revisions and paths, and unverified status. GoldHEN MC4 is opaque encoded material; companion XML names without code payloads are metadata-only. GoldHEN files with PS2-era title IDs are not proof of native PS2 cheat support. GameCube/Wii INI entries keep device sections; Wii U Markdown code blocks remain separated by source revision headings. All original source bytes remain in archive.

**New PS2 and Dolphin archive additions (2026-10-10):** AeroWidescreen contributes another 62 original PS2 PNACH files with author and CRC labels. Dolphin adds 244 confidently classified GameCube/Wii AR/Gecko source INIs to these console indexes, with 34 additional code-bearing originals preserved but deliberately unindexed until their platform family is established. Source IDs and revisions stay exact; compatibility is unverified.

`cargo run --release -p cheatarium-client --bin cheatarium-query -- source --db generated/v1 --platform gamecube --source-record-id 'dolphin-game-cheats:Data/Sys/GameSettings/D43E01.ini' --json`

`cargo run --release -p cheatarium-client --bin cheatarium-query -- source --db generated/v1 --platform ps2 --source-record-id 'aerowidescreen-pcsx2-cheats:Burnout 3 Takedown/SLUS-21050/Button Mapping/SLUS-21050_BEBF8793_buttons.pnach' --json`

**Original native PS2 and additional PS1/PSP/GameCube sources:** The CookiePLMonster collection adds **110 original PS2 PCSX2 `.pnach` files**, 67 PS1 native `.cht`, 4 PSP CWCheat `.ini`, and 2 GameCube INI source records. Source paths and game-title strings remain source claims, not authenticated cartridge/disc build matches. The PS2 index keeps original bracketed patch groups, multiline patch lines, author fields and title information within native text; CWCheat `_C1` marks an **upstream historical enabled state**, not execution by Cheatarium. The PS1 `.cht` files are native cheat collections, not assumed to be Libretro syntax. Original credits and rights remain independently preserved. Example:

`cargo run --release -p cheatarium-client --bin cheatarium-query -- source --db generated/v1 --platform ps2 --source-record-id 'cookieplmonster-console-cheat-codes:PS2/Gran Turismo 4/Adjusted triggers sensitivity/SCUS-97328_77E61C8A_triggers.pnach' --json`

**3DS and Switch Sharkive data:** The 644 3DS and 491 Switch native `.txt` sources are indexed alongside Libretro, with unchanged multiline original cheat sections and original source IDs/ordinals. Upstream file paths provide 3DS title IDs and Switch title/build IDs, **not independently verified human-readable game names, regions or ROM identities**. Switch versions remain separate candidate keys rather than being merged across builds. A missing or unusual build-ID length is preserved verbatim and warned about, not silently corrected. Code entries are unverified and not activated. See [source acquisitions](../sources/README.md) and the [rights audit](SOURCE-AUDIT.md).

`discover` searches advisory game-title groups across **all 56 local platform catalogs** without first choosing a console. Its platform-qualified results include the original candidate key, alternate title hints, source-record references, region/format hints, and counts. Results are sorted by platform and candidate key, then paginated with `--offset` and `--limit`. Consumers can follow one result with `game --platform PLATFORM --game-key KEY` for checksum-checked original-source inspection. Discovery verifies the SHA-256 hashes and byte lengths of `catalog.json` and all 44 game-title / source-ID indexes against the local `distribution.json` manifest before searching. It does **not** read or hash the much larger cheat-entry bundles, authenticate the manifest's publisher, inspect ROMs, or infer cheat compatibility. Its JSON distinguishes verified title-index file checksums from the unauthenticated manifest. The follow-up `game` command verifies the selected platform's complete distribution before reading original cheat sources.

`platforms` lists supported indexed consoles/handhelds without needing `--platform`, with source-file counts, filename-derived game-candidate counts, code/memory counts and artifact paths. Its `--offset`/`--limit` pagination supports small command palettes and GUIs. This is catalog enumeration, **not** proof of any game identity or imported cheat's functionality. A title-search workflow is `platforms` → `games --platform ... --title ...` → `game --game-key ...` → `source --source-record-id ...`.

`game` accepts one **exact advisory candidate game key** (`--game-key`) and joins the separately generated game index to its original per-platform source records. It returns full source IDs, filenames, source/region/format hints, Git blob and upstream licensing provenance, import warnings and per-source counts without dumping all cheat entries. Results use `--offset` and `--limit` with unpaginated totals. Missing/duplicate references, mismatched game keys, and disagreeing source-code/memory counts cause errors; the local distribution is checksum-checked first. The command never promotes a filename match to ROM identity or authorizes activation. Use the returned `source_record_id` with `source` to page individual original cheats.

`entry` resolves a stable original source reference by `--source-record-id` and `--ordinal`, preserving the exact original description, code string, role, enabled bit, and upstream provenance. Unlike pagination or matching code text, the pair addresses an individual original archive entry even when identical codes appear multiple times. Unknown ordinals and duplicate source ordinals are rejected. The command checks the local distribution manifest before returning data; it never activates or claims compatibility for imported codes.

`entries` opens all original cheat entries belonging to one advisory candidate game, across every linked source. Add `--description-contains TEXT` to find source descriptions by case-insensitive substring (trimmed only for matching). The original descriptions, bytes, and entry identities remain unchanged. This is a lexical search, not proof that the described effect works. JSON reports the original query and `description_text_match_only`, along with totals computed **after** applying all filters. Exact source facets `--region-hint`, `--source-id` (upstream collection), and `--declared-format` (explicitly declared source format) narrow results using original labels; missing or inferred values are never promoted to declared formats. Facet comparisons are case-sensitive, and absent or unmatched values return empty results. Source links and aggregate metadata remain checked before filtering. Optional `--source-record-id` narrows to a single source, and `--role code|memory-entry|section-heading` narrows the original entry type. It retains original ordinals, descriptions, raw code text, the original enabled flag, composition/decode metadata, and complete upstream provenance. Pagination uses `--offset` and `--limit`; duplicate code strings are **not merged**, even when repeated within the same archive. Neither this view nor the imported enabled bit authorizes executing a cheat.

`source` inspects one **exact original source record**, identified by `--source-record-id`. It returns the source's filename, advisory title and region/format hints, import warnings, upstream repository/revision/license and Git blob provenance, together with paginated original entries. It preserves device-code fields, native memory entries and section headings separately, including each entry's ordinal, original description, original enable bit and unverified status. `--offset` and `--limit` allow walking large `.cht` files without dumping everything at once. Neither original enabled flags nor imported text authorize execution.

`codes` finds **literal exact-code text** within a chosen platform. It preserves case, spaces and original `+` joins, then reports every matching original source record, ordinal, description, device/region hints and pinned provenance. Optional `--game-key` and `--source-record-id` filters narrow advisory candidates or exact original records; `--offset` and `--limit` paginate matches. This is not normalization or deduplication: exact string matches across sources do not establish equivalent effects, valid ROM revisions, or activation permission. The query never executes cheats.

`repeats` finds identical raw code strings across distinct original source records, within deliberately narrow advisory filename/region/revision/declared-format buckets. These groups are **not** proof that two cheats have the same gameplay effect or are compatible with a particular ROM. Every source ordinal and description remains available. See [repeated-code indexing](REPEATED-CODES.md).

`effects` searches cheat descriptions within the chosen console, optionally narrowed by a filename-derived title, exact source ID, or **explicitly declared** device format. The `--declared-format` option never treats syntax-inferred, unlabeled code as provenance-confirmed Game Genie/Action Replay. Each hit retains the original code entry and complete source provenance; headings are excluded and no cheat is activated. `verify` confirms local files match the distribution manifest, but does not authenticate the manifest itself. Pin a trusted upstream Git commit or future immutable release.

## ROM fingerprint evidence

The `cheatarium-fingerprint` CLI reads a user-selected local file and hashes its **entire original contents**. It may compare that SHA-256 to documented release claims. No ROM data is uploaded or retained and no cheat record is automatically associated with a matching release.

```fish
cargo run --release -p cheatarium-client --bin cheatarium-fingerprint -- --platform snes --file /path/to/local-game.sfc
```

The identity registry is currently empty because filename guesses are not evidence of a ROM checksum. Status `no_evidence` is the correct, safe result until a sourced hash is deliberately added. Review [ROM identity design](ROM-IDENTITY.md).

## No implicit execution

Entries of `role: "code"` preserve native device-code strings; `role: "memory-entry"` preserves address/value metadata; `role: "section-heading"` marks a non-executable label. Codes are imported **unverified**. `source_enabled` means only that the original file marked them enabled.

Complete SNES Game Genie and Pro Action Replay source-code groups may include a derived `snes_decode` object when the source filename identifies the device; source files without a device label may also expose explicitly flagged syntax-only interpretations. The optional `snes_decode` object with ordered `{address_hex, value_hex}` entries, a CPU-bus address-space label, `interpretation_basis` (`declared-file-format` or `code-syntax`), and `compatibility: unverified-cartridge-build`. This is decoding, not execution permission. Undecodable placeholders and unknown formats have no `snes_decode` value. See [SNES codec guide](SNES-CODES.md).

Consumers must decide whether a code is compatible with the exact cartridge build and know how its specific Game Genie, Action Replay, or memory format behaves before applying anything. Nothing in this library activates a cheat automatically.

See [the v1 contract](INDEX-V1.md), [SNES decoding and coverage](SNES-CODES.md), and [distribution specification](DISTRIBUTION.md). The generated [SNES coverage report](../generated/v1/reports/snes-codec-coverage.json) is available for any consumer to inspect.

Nintendo GameCube/Wii U external-source acquisition: `--platform gamecube --source-id leumas-gamecube-wiird-zips --title "NTSC-U"`
or `--platform wii-u --source-id katsumaki-wiiu-xml --title "Twilight Princess"`
search strictly source paths and original pinned link provenance. A
regional GameCube ZIP path is not a browseable game-by-game code list until
its members are inspected separately. All returned codes remain excluded.

### Nintendo GameCube regional WiiRD member lookup

Run `python3 tools/query_gamecube_wiird_members.py --query GZLE01 --region NTSC-U --limit 10` or omit the region to search all 375 source ZIP member filename hints. Results provide source path, region and original pinned parent ZIP URL; they are **not** local code payloads, ROM identities, tested cheats or license grants. Full regional counts are verified by a code-free synthetic test. The separate checksum-verified external catalog still indexes the three original regional ZIP containers, not their unarchived members.

### Private Nintendo original source cache

Use `python3 tools/hydrate_nintendo_bulk.py --fetch` to explicitly materialize
**375 GameCube WiiRD cheat .txt files** into region-specific directories and
**13 Wii U original XML/JSON cheat documents** into a user-owned,
non-repository local cache. The default is
`$XDG_DATA_HOME/cheatarium/nintendo-originals` (typically
`~/.local/share/cheatarium/nintendo-originals` on EndeavourOS).
Use `--destination /custom/absolute/path` to select another directory.
The original source revision and full Git blob SHA-1 are verified before
extraction; all GameCube ZIP member paths/sizes are cross-checked against
the pinned 375-member provenance catalog; the 388 output files receive
SHA-256 digests in a private `manifest.json`. A new cache is published
only once **all sources validate**, so interrupted downloads leave no
partially visible cheat library. Existing valid caches are reused without
re-downloading. `python3 tools/hydrate_nintendo_bulk.py --check`
checks an already populated cache fully offline and does not access the
network. If an existing cache is corrupt, the tool refuses to overwrite it.

The cache cannot be placed inside Cheatarium's Git working tree; its
original cheat data must **not** be committed or published. Only the source
files and local verification manifest are created, never game images,
firmware or executables. The cache contains raw native WiiRD/GBAtemp/uCodes
strings, *not* normalized or independently execution-verified Cheatarium
code entries, and source-author redistribution rights remain unresolved.
GameCube source ZIPs have no effect-level emulator QA. A GitHub Actions
integration test fetches the same pinned original files into an ephemeral
runner's private path, verifies the manifest twice (including an offline
`--check`), then discards the cache without artifact publication.

### Wii native Gecko code source paths

External reference search now includes **29** pinned original code files across four Wii original-source families: for instance, `cheatarium-query external --db generated/v1 --platform wii --title RUUE01v0 --json` matches the explicit City Folk revision. The game ID alone is not enough for City Folk; do not merge `RUUE01v0` and `RUUE01v1`. This is source filename search and source provenance, **not** a version verification or public code-body import.

### Private Wii original-code acquisition

The newly pinned **29 original Wii Gecko code files** (9 City Folk/Deluxe,
12 Metroid Prime/Trilogy, 3 Metroid: Other M, 5 Taiko) can be retrieved
as actual native `.ini` and `.txt` source files into a private local cache:

`python3 tools/hydrate_wii_bulk.py --fetch`

The default destination is
`$XDG_DATA_HOME/cheatarium/wii-originals` (typically
`~/.local/share/cheatarium/wii-originals` on Linux). The tool writes
`wii/<original-source-id>/<upstream-native-path>` and a private
`manifest.json`. **Never combine differing disc revision variants**:
City Folk `RUUE01v0.txt` and `RUUE01v1.txt` are distinct source files;
Metroid Prime `R3ME01_mp1.ini` and `R3ME01_mp2.ini` are separate source
variants. Set `--destination /some/other/private/path` if needed.

`python3 tools/hydrate_wii_bulk.py --check` performs **offline** Git
blob SHA-1 checks against the repository's pinned original files, plus
SHA-256 verification of the user's cached contents. `--fetch` verifies
and reuses an existing complete cache; incomplete/corrupt existing
destinations are refused rather than overwritten. New caches are
published only after all 29 real upstream source files have been
downloaded and verified, in a staging directory outside the public
repository. No ROMs, firmware, game files, third-party code bodies,
or cache payloads are written to the Git tree, CI artifacts, or
versioned Cheatarium distribution.

**Caveat:** The raw native files are available for local study/use, not
yet normalized into Cheatarium's index or proved compatible with any
exact game revision. Original third-party Gecko code ownership is
rights-unresolved, so private local acquisition does not grant
permission to redistribute those codes. No game effects have been
empirically verified by Cheatarium.

### Wii Gecko native-code audit without source-body export

After explicit private acquisition, `python3 tools/audit_wii_gecko_native.py --cache ~/.local/share/cheatarium/wii-originals` verifies the original files against pinned Git SHA-1s and outputs **aggregate code-group and hexadecimal instruction-line counts**. The committed [aggregate audit](../research/wii-gecko-native-syntax-audit-v1.json) contains **45 native Gecko group headers and 5,611 instruction-pair lines** across the 29 source files, with source-family counts. It exports no code text, per-game identifiers, fingerprints, or original cheat bytes; do not interpret its lines as separate executable cheat effects or auto-enable them.

### Local Wii game/revision-specific Gecko group search

Once the private original files have been acquired with
`python3 tools/hydrate_wii_bulk.py --fetch`, all **45** source-native Wii
Gecko groups are searchable **offline**, with the original author notes
and source-version warnings preserved:

- `python3 tools/query_wii_gecko_local.py --game-title "Metroid"` lists source-game families and native group names.
- `python3 tools/query_wii_gecko_local.py --game-id RUUE01` lists two **distinct** City Folk variants (v0 and v1). Nothing is selected automatically.
- `python3 tools/query_wii_gecko_local.py --game-id R3ME01` lists four separate Metroid Trilogy source variants (base/mp1/mp2/mp3).
- `python3 tools/query_wii_gecko_local.py --game-id RUUE01 --revision 0 --show-code --group-index 1` deliberately displays the selected code text from the private **local source file**, retaining every Gecko native instruction pair. Use `--variant mp1` to disambiguate Prime-specific source files and `--title` to filter group titles/creator notes.

The default JSON output includes **only metadata**, native group titles,
original upstream URL and Git provenance, original source author notes,
line counts and optional source-enabled flags. Original executable code
appears only when `--show-code`, `--game-id`, and a one-based
`--group-index` explicitly identify one original source file. If
multiple revisions or variants remain, the code-view request fails
rather than guessing a build. Even a title search cannot silently
select an otherwise ambiguous revision.

This lookup does **not** install, activate, copy, or export cheats to an
emulator; it is read-only and it does not authenticate a user's actual
game disc version or executable. The upstream source game-family title
and version stem are hints, not independent ROM identity evidence.
No third-party code bodies are added to public Git or CI artifacts;
license grants and real hardware/emulator behavior remain unverified.

### Private Wii Gecko consumer sidecar (locally projected original code)

The acquired private Wii originals can now be projected into one **local,
version-preserving JSON sidecar** with 45 selectable native cheat groups
and 5,611 Gecko instruction-pair lines. All original code bytes remain
strictly outside the public Cheatarium repository, releases and CI
artifacts:

`python3 tools/export_wii_gecko_local.py --write`

The default destination is
`$XDG_DATA_HOME/cheatarium/wii-local-sidecar/codes.json`
(typically `~/.local/share/cheatarium/wii-local-sidecar/codes.json`).
Use `--verify` to compare the existing JSON **exactly** against freshly
reparsed and Git-blob-verified original source files. Use
`--output /your/private/codes.json` to choose another path, and
`--replace` only if deliberately replacing an existing local sidecar.

Each entry retains original game/disc-ID **claim**, source revision token,
source variant (including MP1/MP2/MP3), code-group title and author notes,
original source filename, original source Git SHA-1, pinned upstream
provenance URL, unchanged native code instructions, source-enabled marker,
and compatibility warning. Crucially, all projected entries set
`enabled_for_execution: false`, `game_build_authenticated: false`,
`gameplay_verified: false`, and
`original_author_redistribution_rights_cleared: false`.

This is a **consumer-readable local representation**, not emulator
activation, authenticated disc matching, a tested cheat-effects database
or authorization to redistribute the upstream authors' codes. The
automatic end-to-end CI integration builds and verifies an ephemeral
private sidecar but uploads **only aggregate code counts**, never the
sidecar or its original third-party cheat text.

### Native Rust consumer for the private Wii sidecar

After the private sidecar is generated locally, Cheatarium's Rust client
can read it **without network, ROM access, or Python during lookup**:

`cargo run --locked -q -p cheatarium-client --bin cheatarium-wii-local -- --game-title Metroid`

`cargo run --locked -q -p cheatarium-client --bin cheatarium-wii-local -- --game-id RUUE01 --json`

`cargo run --locked -q -p cheatarium-client --bin cheatarium-wii-local -- --game-id RUUE01 --revision 0 --show-code --group-index 1`

The binary defaults to
`$XDG_DATA_HOME/cheatarium/wii-local-sidecar/codes.json`; use
`--index PATH` to read a different private sidecar. The format has
**strong fail-closed guards**: expected 29 source files, 45 native
groups and 5,611 instruction-pair lines; duplicate group identities,
invalid hexadecimal instruction pairs, enabled executable entries,
unverified source schema or version ambiguity are rejected. The Rust
reader validates format and safety invariants but does **not**
reconstruct independent Git SHA-1 evidence by itself; use
`python3 tools/export_wii_gecko_local.py --verify` when full local
source-byte verification is needed. Source compatibility labels are
never authenticated disc identities, and **no emulator is controlled**.

## Eden Linux read-only library discovery (experimental)

The `cheatarium-eden-discover` binary reads Eden's Qt `[UI]` `Paths\\gamedirs\\<N>\\path` settings (and alternative `[Paths]` layouts) and lists native Switch container filenames without opening NSP/XCI/NCA contents or reading emulator secrets. It also reads Eden's cached `game_list/<TITLEID>.appname.txt` names and recognizes `game_list/<TITLEID>.pv.txt` labels from the XDG cache directory; those historical cache records provide useful additional title IDs without accessing the ROM, but cannot prove the game remains installed. It associates only **complete 16-hex title-ID labels already in filenames** with the checksum-checked Cheatarium Switch candidate index; the tool also looks up rights-limited third-party **source references** (not code payloads). A filename ID is never proof of authenticated game identity; indexed Build IDs are candidate cheat-version labels, **not the installed game's Build ID**. The tool makes no changes to Eden, games, saves, configs or Cheatarium indexes.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --json
```

For portable Eden installs or custom layouts, use `--config /path/to/qt-config.ini`, `--cache-dir /path/to/eden/cache/game_list`, and/or repeat `--game-dir /path/to/game/files`. Supply a known ID such as `--title-id 0100000000010000` for a **separate explicit lookup** when the ROM filename doesn't expose the ID. The JSON report distinguishes `explicit_lookup`, `filename_only`, `filename_and_eden_cache`, `eden_cache_only_unverified_installed`, and unresolved filenames. It reports candidate build IDs without treating them as compatible with the installed executable.

Supported defaults: `$XDG_CONFIG_HOME/eden/qt-config.ini` or `~/.config/eden/qt-config.ini` for configuration, and `$XDG_CACHE_HOME/eden/game_list` or `~/.cache/eden/game_list` for Eden's Title ID cache; the read-only parser accepts namespaced `[UI]` Qt `Paths\\gamedirs\\1\\path` entries and their `deep_scan` flags, plus older standalone `[Paths]` entries. Only configured plain directories are scanned, without following symlinks, to a finite depth and count. Eden can also use portable `user` directories; pass its config explicitly. `.pv.txt` contains Eden's patch/version display, **not the executable Build ID**, and only its filename is used as Title ID evidence. A cached ID may be stale and is not linked to a particular NSP/XCI filename unless other evidence is present. Unidentified filenames, custom installations, and game titles encrypted inside Nintendo containers remain unresolved; proper automatic installed title/version discovery from emulator metadata is future work. **Do not mistake a missing filename ID for a game missing from Eden.**

External Switch link-only source coverage is scanned once for the entire detected title-ID set, rather than reopened per game. Exporter work is deliberately **not** enabled in this phase. Eden uses `<mod directory>/<effect label>/cheats/<BuildID>.txt` for separate add-on toggles; cheat files may be enabled by default, so the future writer must guarantee disabled-by-default behavior, explicit Build ID confirmation, backups, and no overwrites before it can be exposed as a safe operation.


## Eden native Switch cheat preview, staging, and direct installation

Use `cheatarium-eden-plan` to **preview** independent per-cheat Eden add-ons for an **explicit Title ID + archived Build ID**. The scanner verifies the local Switch artifact checksums against the generated Cheatarium distribution and selects original `sharkive` cheat sections **only for that exact pair**. It never chooses a Build ID from title matching alone. The selected Build ID is **not proof** of the installed executable's version.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --title-id 0100000000010000 --build-id B424BE150A8E7D78 --json
```

The plan lists each original section's name, source record and ordinal, code-line count, a source-text checksum, a lexical syntax status, and its proposed individual path under `$XDG_DATA_HOME/eden/load/<TITLEID>/<readable cheat name>/cheats/<BUILDID>.txt` (with `~/.local/share` as the standard XDG default). `--mods-root` customizes the **displayed destination** for portable Eden installs. Destination paths are previews, not directory creation. Each entry retains independently traceable source provenance; duplicate labels receive a numeric suffix and do not collide.

**Preview mode** (without `--stage-dir` or `--install`) is read-only: it does not write or check existing mod paths, enable cheats, inspect executable Build IDs, or verify cheat effects. Its `installation_allowed: false` and `installed_build_id_verified: false` mean that compatibility remains unverified, **not** that explicit file installation is unavailable. The archived Sharkive text is not gameplay-tested; a lexical hex check is not execution validation. Metadata-only external references never become cheat payloads.

### Isolated staging export

To create a **new** directory of separately packaged, non-installed native Switch cheats, explicitly supply `--stage-dir`. The path **must be absolute, must not already exist**, and its parent must already exist without symlink traversal. The staging writer refuses destinations under Eden data, mods, config or cache directories. Staging is an optional action; without `--stage-dir`, the command remains a pure JSON preview.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --title-id 0100000000010000 --build-id B424BE150A8E7D78 --stage-dir /tmp/cheatarium-staging-odyssey --json
```

The writer requires at least one exact archived Sharkive source record for the selected Title ID + Build ID. It refuses staging if **any** selected source section has an unsafe name or an unsupported lexical opcode shape (no partial selection of unresolved source text). It preflights all source records and relative paths before creating anything, uses create-new writes so it never overwrites files, and writes a `cheatarium-staging-manifest.json` containing original source references, individual relative cheat paths, SHA-256 values, and explicit **not installed / not compatible-verified** flags. Repeating the same operation against the existing directory is an error, not an overwrite. If a filesystem error happens after directory creation, a partial staging directory can remain and must be inspected or discarded manually.

**Staged cheats are not installed or active.** Isolated staging never modifies Eden, while the separate `--install` option performs installation with per-addon disabled-by-default configuration. No rights-unresolved metadata-only external source is materialized.


### First: inspect the complete proposed changes (strictly read-only)

For normal operation, run the code already in this repository with **no Title ID or Build ID**, and with **no installation**:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --dry-run --report
```

This prints every candidate game found in Eden's registered folders and game-list cache, its display name and discovery evidence, every matching archived build and cheat name, the originating archive, **exact target path, SHA-256, byte count and full proposed cheat-file contents**, plus the proposed `[DisabledAddOns]` section from Eden's Qt configuration. Existing identical disabled files and conflicting mod directories are marked separately. No files are staged, created, installed or enabled, and `qt-config.ini` is never changed. The program only reads the existing sources and creates its proposed files and settings **in memory**. Use `--dry-run --json` instead for the machine-readable equivalent; both modes are entirely non-writing. `--dry-run` explicitly **refuses** `--install` or `--stage-dir`.

A dry-run report is **not authorization to install** and makes no claim that any archived Build ID matches the installed game's executable. A future installation should happen only after the user has reviewed the complete report.

For a narrower **historical-main-only dry run**, use the same Rust CLI with `--dry-run --historical-main-only --report`. It reads Eden's existing launch log **and**, if present, Cheatarium's private saved main Build ID history at `~/.local/state/cheatarium/eden-build-history.json`. It selects an archived cheat Build ID only when **exactly one distinct historical main executable Build ID** exists across both sources for that title. Missing or conflicting observations are skipped rather than guessed, and weak cheat-file lookup observations are never treated as main executable evidence. Both sources are read-only in this mode; `--history-file` can override the private history path. It still prints every selected file's exact intended contents and destination, plus reasons for skipped archived builds. This mode is read-only and writes **nothing** to Eden. The report labels the selection scope `unique_historical_main_nso_build_only_not_current_verified`.

The corresponding explicit `--install --historical-main-only` command uses the **same selection rule**, but must only be used after the dry-run has been reviewed and approved. Unlike `--install` alone, it will not install all archived variants for the discovered titles. Because Eden can rotate its log and game updates can change, rerun the preview immediately before any approved installation. **A historically observed main NSO Build ID does not prove the current installed executable version or cheat effectiveness.** Neither command launches games, reads ROM executable bytes or determines the currently active update.

### Eden file-only installation (no Qt settings changes)

For users who prefer to manage active cheats directly in Eden, Cheatarium has a separate `--files-only` mode. It creates the normal native Eden layout `load/<TITLEID>/<Cheatarium addon name>/cheats/<BUILDID>.txt` **without editing or backing up** `qt-config.ini`. The tool still **reads** the Qt configuration during game discovery to locate Eden's registered game directories, but does not write it. Eden's own [cheat guide](https://github.com/eden-emulator/mirror/blob/master/docs/user/UsingCheats.md) says newly discovered cheats are enabled by default and can be toggled individually in Configure Game → Add-ons.

For a narrowly scoped, non-writing preview of the previously observed Super Mario Odyssey build, run:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --historical-main-only --only-title-id 0100000000010000 --files-only --dry-run --report
```

After reviewing the exact files and source Build ID, the matching, explicitly selected file-only operation is:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --historical-main-only --only-title-id 0100000000010000 --files-only --install --json
```

`--files-only` deliberately **requires** both the conservative `--historical-main-only` Build ID policy and `--only-title-id`; it refuses all-title/all-archive installs and guesses. Close Eden before running an installation. Existing mod directories are protected from overwrites; identical complete file-only installs are recognized on a repeat run. No config backup is needed because this operation does not write the config. If an I/O error interrupts file creation, some new directories may remain and should be reviewed, never silently overwritten. The source cheat effects are not verified, game updates can change the executable Build ID, and enabling several independent cheats can cause conflicting effects. Manage their checkboxes before launching the game if needed.

### Readable Eden add-on names and safe legacy rename

Cheatarium now uses the original human-readable cheat descriptions as folder names, e.g. `Infinite Health Save 1`, `9999 Coins Save 1`, `Refill Health Save 1 (L+R)`, and `Max Purple Coins Save 1`. Duplicate descriptions get a numeric suffix. Internal Cheatarium ordinals and hashes remain in source records and reports, **not the Eden Add-ons UI**.

For installations from the older `Cheatarium-0000-...-<hash>` naming scheme, run a **read-only migration preview** after closing Eden:

```sh
cheatarium-eden-plan --db generated/v1 --historical-main-only --only-title-id 0100000000010000 --files-only --rename-installed --dry-run --json
```

To apply the exact verified rename, omit `--dry-run`:

```sh
cheatarium-eden-plan --db generated/v1 --historical-main-only --only-title-id 0100000000010000 --files-only --rename-installed --json
```

The migration requires unambiguous historical executable Build ID evidence, verifies every existing original directory and exact cheat-file bytes against the indexed source, checks for unexpected extra files and new-name collisions, and refuses to rename if a legacy addon name is recorded as disabled in Eden's settings. It changes **directory names only**, not cheat bodies, Qt configuration, user saves, or other mods. On an intermediate move error it attempts to reverse previously completed renames and reports if that recovery was incomplete. Re-running after success is idempotent. The existing legacy folders are not copied or duplicated; they are moved to clean names.

### Direct on-demand installation into Eden (no IDs to look up)

**Close Eden completely**, then run the normal CLI **without** any Title ID or Build ID arguments:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --install --json
```

`--install` with no IDs automatically reads Eden's `qt-config.ini` registered game directories (recursively when enabled) and its `game_list` cache. It identifies candidate **base-game** Title IDs from known file labels and existing cache file names; update and DLC labels are not mistaken for base games. It then finds **all available archived Sharkive Build ID variants** for those titles. The user does not need to identify the executable Build ID: each code file is installed under its own archival Build ID-specific filename; **Eden will look up only the matching Build ID filename** for the executable it actually starts. The installer does **not** claim an archival Build ID matches the running version.

All newly installed per-cheat add-ons are **disabled by default**. Immediately before replacing the Qt config, the installer now creates and verifies a separate, create-new, permissions-preserving backup named `.qt-config.ini.cheatarium-backup-<pid>-<serial>` beside the original config. The `eden_config_backup_path` installation-result field records its exact location. If backup creation fails, the original Qt config is left unchanged and installation refuses to proceed. Cheatarium writes the relevant `[DisabledAddOns]` settings in `~/.config/eden/qt-config.ini` before publishing their cheat files under `~/.local/share/eden/load/<TITLEID>/Cheatarium-.../cheats/<BUILDID>.txt`. Existing add-ons and unrelated Eden settings are preserved, with no overwrites. Repeated automatic runs recognize exact unchanged Cheatarium files that remain disabled and skip them; a collision with a different file is refused. Missing archive coverage or invalid source text is listed in the report rather than guessed or silently installed. Eden must be fully closed while installing.

To **preview automatically without writing anything**, use `--auto` without `--install`. Portable setups may specify `--config`, `--cache-dir`, or `--mods-root` to match Eden's locations. The older manual `--title-id TITLE --build-id BUILD --install` remains optional and is **not required for normal use**. `--stage-dir` is the separately selected isolated staging mode.

Cheat compatibility and the current installed executable Build ID remain unverified. The command writes no ROMs, saves, or encryption keys, and does not activate any cheats by itself. A partial I/O failure can leave some disabled add-ons on disk. After installing, individual cheats can be enabled from Eden's per-game add-on controls.

## Eden historical main Build ID observations (read-only)

The `cheatarium-eden-builds` CLI inspects Eden's **existing** plaintext log at `$XDG_DATA_HOME/eden/log/eden_log.txt` (default `~/.local/share/eden/log/eden_log.txt`). Eden emits game-launch lines with a 16-digit Title ID and `Querying NSO patch existence for build_id=..., name=main` records. Depending on Eden's version, the main NSO query can appear **before or after** the `Core::Load` line. Cheatarium pairs nearby canonical core.cpp loading lines in either order and never treats non-main NSO queries as executable Build ID evidence. Cheatarium associates those records **only within the same observed launch context**, normalizes the first 16 hex digits of the main NSO Build ID, and joins the observed pair to the archived Switch cheat candidate index.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-builds -- --db generated/v1 --json
```

Use `--log /absolute/path/to/eden_log.txt` to inspect a deliberately chosen historical log file. Input is bounded to 32 MiB, read only, and symlinks are rejected. The JSON report does **not** reproduce arbitrary log lines, game paths, user names, or other log details. It reports Title ID, shortened main Build ID, original hex length, per-log launch sequence number, archived-match status, and unassociated main-build count. It also reports how many Core launch contexts and `name=main` queries the existing log contained, so an empty result can be distinguished from a log that never captured a game launch. Separately, `direct_cheat_lookup_observations` records explicit Title ID / Build ID pairs from Eden's `No cheats file found` or `Failed to read cheats file` messages. Those indicate cheat-file lookups, **not** necessarily a `main` executable query; no Build ID is promoted to verified main or installed merely from a lookup. Duplicate lookups are collapsed. If the log does not exist, an empty report is returned; an empty report is **not** evidence there are no games.

**Limits:** A historical main NSO Build ID is evidence of what Eden previously attempted to run, not proof of what is currently installed or proof that a cheat works. A single log may contain many games or old versions. Multiple builds for one Title ID are explicitly reported as ambiguous; contextless records are discarded. `current_installed_build_verified`, `compatible_cheats_verified`, and `installation_allowed` remain false. No ROMs, keys, saves, or emulator settings are accessed; no cheats are installed or activated. The staging/export process remains separate and opt-in.

### Compact inventory from Eden's registered folders

`cheatarium-eden-discover --summary --json` produces a compact view derived from **the same configured Eden roots**, existing cache labels and filenames as the full discovery report:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --summary --json
```

The summary identifies candidate base games by standard Title ID label shape, possible update containers using Eden's own `GetUpdateTitleID(base) = base | 0x800` rule, and likely DLC files grouped by user directory labels such as `dlc/`. It lists filenames with no embedded Title ID separately, rather than guessing their identities from adjacent files. It does not report each individual DLC Title ID as an independent playable game.

Every classification is explicitly **advisory**: an NSP filename may be wrong; a cached title may be stale; a directory labeled `dlc` does not verify an add-on; and a downloaded update in a game folder **does not mean that Eden has installed or enabled that update**. Likewise, neither `qt-config.ini` nor the game-list cache records the currently loaded executable Build ID. The summary doesn't open game containers or touch Eden, and it leaves all install/build compatibility flags false. The summary also reads Eden's saved `[DisabledAddOns]` entries and its existing `game_list/<TITLEID>.pv.txt` patch-display caches. It reports explicitly disabled update versions (`Update@<version>`), a generic `Update` disable marker if present, the saved add-on names, and cached lines such as `Update (1.3.0)` and `[D] Update (1.2.0)`. These are two **distinct historical/configuration signals**: the cache can be stale, disabled settings are not a list of installed updates, and an update in the list does not identify a currently executed Build ID. Neither signal enables installation or compatibility claims.

The older full JSON format is unchanged unless the `--summary` flag is specified.


### Optional bounded NSP header inspection for unresolved filenames

If Eden's registered folder contains `.nsp` files with no embedded Title ID (e.g. `Mario Kart 8 Deluxe.nsp`), Cheatarium can optionally inspect the **outer PFS0 header only** of up to 24 unresolved NSP files, each capped at **256 KiB of header bytes**:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --summary --inspect-nsp-headers --json
```

By default, `--summary` continues to read **zero game-file bytes**. The additional option reads **exactly** the outer NSP `PFS0` header and entry-name table, not the following ticket or encrypted NCA payload, using Eden's documented PFS0 layout with strict file-size and entry-count checks. It can sometimes derive a **candidate Title ID from a ticket's 32-hex filename** (first 16 digits), even if the NSP's human-readable basename contains no Title ID. Ticket **contents**, NCA payloads, ROM encryption keys, saves, and executable code are never parsed, decrypted, exported, or written. Unsupported or malformed files are reported as such; they are not guessed from neighboring files. Header reads are counted in `game_file_bytes_read` and `inspected_nsp_file_bytes_read`. The original full discovery output is unchanged.

If exactly one ticket filename provides a plausible base-game or `base | 0x800` update Title ID **and its base title is already represented by Eden's cache or another labeled game file**, the summary now adds an `advisory_nsp_title_associations` entry linked to that existing base-game record. `ticket_filename_labeled_base_files` and `ticket_filename_labeled_update_files` track these hints separately from stronger filename-labeled counts. `unresolved_after_nsp_header_inspection` retains filenames with missing, ambiguous, or unmatched ticket hints (including NSPs like Skyward Sword HD with no `.tik` entry). Files with multiple candidate ticket Title IDs are never arbitrarily assigned. The original `unresolved_filename_title_ids` and `cached_title_ids_without_labeled_base_file` evidence remain unchanged to avoid concealing how matches were obtained.

A title found in a ticket **filename** is weak, unauthenticated evidence; it does not prove the NSP's actual program identity, its type (base/update/DLC), or its executable Build ID. NSPs may not contain a ticket, and Switch update selection may depend on Eden's other installed/registered content. `executable_build_id_found`, `installed_build_id_verified`, and install compatibility remain false. The purpose is to improve offline discovery incrementally, **not** bypass encryption or silently enable cheats.


### Comprehensive audit of every Eden-registered game file

The earlier `--inspect-nsp-headers` option is intentionally a *small diagnostic* (at most 24 NSPs without Title IDs); it is **not** an audit of the user's Switch library. To inspect **every recognized game file found under Eden's registered roots**, including DLC and files whose names already contain Title IDs, use:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --summary --audit-all-files --json
```

The `--audit-all-files` option is separate from, and mutually exclusive with, the old `--inspect-nsp-headers` option. It reports `audit_file_count` and one `files[]` row for **each recognized file in `game_files`**, together with `header_inspections_attempted`, `listed_only_files`, `header_status_totals`, and `total_header_bytes_read`. NSPs are examined via the bounded outer PFS0 filename table, including all DLC NSPs. For XCIs it checks the unencrypted `HEAD` gamecard magic at the offsets Eden's parser accepts, without opening partitions. Formats requiring additional encryption or compression support (`.nca`, `.nsz`, `.xcz`, `.nro`) are **listed with explicit non-inspected reasons**, not silently counted as parsed. No key or decrypted executable content is accessed. A matching filename and ticket filename are advisory evidence only; conflicting hints are flagged instead of arbitrarily resolved. In full-audit mode, the same already-read header evidence also populates the embedded compact inventory's ticket-derived base/update counts, missing-title associations and unresolved-after-inspection list. No second file read is needed. `inspected_nsp_file_bytes_read` counts all NSP header reads in full-audit mode, not just previously unresolved NSPs.

Traversal uses the recursive setting already present in Eden's `qt-config.ini`. The directory cap has been raised from 5 to 64 levels. The audit's `traversal` counters account for directories and entries visited, non-game files, symlinks, special files, and directories skipped due to depth or disabled deep scan. There is a 50,000-entry / 20,000-recognized-file inventory limit and a conservative **128 MiB aggregate header-read limit**; when reached the output explicitly reports incomplete coverage and records uninspected files. `complete_registered_file_inventory` is only true when all configured roots were accessible, no scan cap was reached and no header-read budget was exhausted. This covers recognized files under configured roots, **not** all files elsewhere on a disk or files with arbitrary unrelated extensions.

The report does **not** claim that base games/updates/DLC are installed, that a particular update is active, or that a game's executable Build ID matches an archive. The operation is fully read-only and makes no modifications to Eden. Unlike `--summary` alone, audit mode will read bounded outer file metadata, with the actual number of bytes reported.


### Per-game Eden cheat Build ID readiness (read-only)

After the regular discovery is working, `cheatarium-eden-discover --summary --build-readiness --json` joins **the same Eden-registered game-directory inventory and existing game-list cache** to the bounded **historical Eden log parser** used by `cheatarium-eden-builds`. Readiness now **automatically captures** newly observed minimal Build ID evidence in Cheatarium's private state. Use `--read-only` or `--no-capture-build-history` to inspect without writing even that private history. No second registration of game directories, and no game container bytes are read by this mode.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --summary --build-readiness --json
```

The optional `--log /path/to/eden_log.txt` supplies a specific historical log (default `$XDG_DATA_HOME/eden/log/eden_log.txt`, or `~/.local/share/eden/log/eden_log.txt`). It returns the existing inventory plus a `build_readiness` object with **one entry per candidate base Title ID**, archived Build ID options, uniquely observed **main** executable Build IDs, historical archive matches, weak title-paired cheat lookup hints, and explicit states: no log, no main observation, weak lookup only, single historical archive match, historical nonmatch, or ambiguous multiple historical builds. The output includes total Core launch contexts, main patch queries, orphan main queries, and log Title IDs outside the current inventory. Duplicate queries within a launch are collapsed; multiple *distinct* historical builds for the same title remain ambiguous.

Every reported match is historical and unverified as a representation of the current installed/running game. Updates in ROM folders and cached labels are not evidence of the active update or Build ID. A `No cheats file found` lookup may be from a non-main executable and is **never promoted to a main executable Build ID**. `current_installed_build_verified`, `compatible_cheat_verified`, `automatic_installation_allowed`, and the overall `installation_allowed` remain false. Nothing is installed, activated, or written to Eden. The default readiness invocation may write **only** a minimal, private Cheatarium history record when new log observations are present; use `--read-only` for an entirely read-only report.

Use `--summary --audit-all-files --build-readiness` only if an additional complete header audit is deliberately desired; the simpler `--summary --build-readiness` avoids rereading NSPs. The default summaries are unchanged unless `--build-readiness` is explicitly selected.


### Preserve minimal Eden Build ID evidence across log rotation

The historical Eden log may be overwritten on emulator restart. When you run `--summary --build-readiness`, Cheatarium now automatically preserves **only** observed 16-digit Title IDs, 16-digit Build IDs, evidence kinds (`main_nso` vs `cheat_lookup`), and *capture* timestamps in a private local JSON file (not the full log). No save files, keys, ROM contents, game filenames, log lines, or emulator settings are copied.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --summary --build-readiness --capture-build-history --json
```

The default history file is `$XDG_STATE_HOME/cheatarium/eden-build-history.json` (otherwise `~/.local/state/cheatarium/eden-build-history.json`). To isolate a test, supply `--history-file /absolute/path/to/history.json` along with `--log /absolute/path/to/eden_log.txt`. The first capture uses an atomic temporary-file replacement, a single-writer lock, and private permissions on Linux. Repeated capture of an unchanged log is idempotent. Corrupted histories, symlink destinations, disallowed Eden directories, oversized records, and conflicting writers are refused rather than silently repaired or overwritten. **Normal readiness runs automatically capture** fresh, bounded Eden Build ID observations to Cheatarium's private state; no separate capture command is required each time. `--capture-build-history` remains supported for backward compatibility. Pass `--read-only` (alias `--no-capture-build-history`) to disable all state writes and only *read* existing history; this takes precedence even when `--capture-build-history` is also present. A plain discovery scan without `--build-readiness` remains fully read-only. No repeat registration of Eden game folders is necessary.

After Eden rotates or clears its log, the report retains old records as `saved_historical_main_builds` or `saved_cheat_lookup_build_ids`, separately from `historical_main_executable_builds` observed in the current log. Statuses beginning `stored_` explicitly mean **saved historical evidence**, not a fresh game launch. `first_captured_unix_seconds` and `last_captured_unix_seconds` mark **when Cheatarium captured a log**, not when a game was played. `saved_history_written_this_run` exposes the optional state write; the report's `read_only` field becomes false *only if* that write actually happened. `emulator_files_written` remains zero. A rotated or empty log does **not** clear the saved observations.

**Limitations:** automatic history capture happens **when Cheatarium's readiness command runs**, not continuously when Eden launches a game. It cannot recover launches erased before the first capture, does not run as a background service, and cannot prove what Build ID Eden would run now. The stored history never enables cheat installation or elevates weak cheat-file lookup hints into verified main executable evidence.
