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


## Eden native Switch add-on preview (read-only, experimental)

Use `cheatarium-eden-plan` to **preview** independent per-cheat Eden add-ons for an **explicit Title ID + archived Build ID**. The scanner verifies the local Switch artifact checksums against the generated Cheatarium distribution and selects original `sharkive` cheat sections **only for that exact pair**. It never chooses a Build ID from title matching alone. The selected Build ID is **not proof** of the installed executable's version.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --title-id 0100000000010000 --build-id B424BE150A8E7D78 --json
```

The plan lists each original section's name, source record and ordinal, code-line count, a source-text checksum, a lexical syntax status, and its proposed individual path under `$XDG_DATA_HOME/eden/load/<TITLEID>/Cheatarium-<ordinal>-<name>-<digest>/cheats/<BUILDID>.txt` (with `~/.local/share` as the standard XDG default). `--mods-root` customizes the **displayed destination** for portable Eden installs. Destination paths are previews, not directory creation. Each entry retains independently traceable source provenance; identical labels will not collide.

The command **does not write**, check existing paths, enable cheats, create mod folders, inspect game executable Build IDs, or verify cheat effects. It intentionally returns `installation_allowed: false` and `installed_build_id_verified: false`. The currently archived Sharkive text is not gameplay tested; the lexical hex check is not a compatibility validator. The future writer must preserve existing files, validate the exact installed executable Build ID, atomically handle Eden's per-title disabled add-on settings, and refuse to activate anything until the user deliberately enables it. Metadata-only rights-unresolved external sources are never included as executable cheat payloads.

### Isolated staging export

To create a **new** directory of separately packaged, non-installed native Switch cheats, explicitly supply `--stage-dir`. The path **must be absolute, must not already exist**, and its parent must already exist without symlink traversal. The staging writer refuses destinations under Eden data, mods, config or cache directories. Staging is an optional action; without `--stage-dir`, the command remains a pure JSON preview.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-plan -- --db generated/v1 --title-id 0100000000010000 --build-id B424BE150A8E7D78 --stage-dir /tmp/cheatarium-staging-odyssey --json
```

The writer requires at least one exact archived Sharkive source record for the selected Title ID + Build ID. It refuses staging if **any** selected source section has an unsafe name or an unsupported lexical opcode shape (no partial selection of unresolved source text). It preflights all source records and relative paths before creating anything, uses create-new writes so it never overwrites files, and writes a `cheatarium-staging-manifest.json` containing original source references, individual relative cheat paths, SHA-256 values, and explicit **not installed / not compatible-verified** flags. Repeating the same operation against the existing directory is an error, not an overwrite. If a filesystem error happens after directory creation, a partial staging directory can remain and must be inspected or discarded manually.

**Staged cheats are not active and must not be copied into Eden yet.** Native add-ons may be enabled by default when placed under Eden's `load` directory, and the tool cannot currently verify the installed executable's Build ID or apply disabled-add-on settings. This stage does not touch Eden and is not an installation command. No rights-unresolved metadata-only external source is materialized.


## Eden historical main Build ID observations (read-only)

The `cheatarium-eden-builds` CLI inspects Eden's **existing** plaintext log at `$XDG_DATA_HOME/eden/log/eden_log.txt` (default `~/.local/share/eden/log/eden_log.txt`). Eden emits a game launch with a 16-digit Title ID and later emits a `Querying NSO patch existence for build_id=..., name=main` log record. Cheatarium associates those records **only within the same observed launch context**, normalizes the first 16 hex digits of the main NSO Build ID, and joins the observed pair to the archived Switch cheat candidate index.

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-builds -- --db generated/v1 --json
```

Use `--log /absolute/path/to/eden_log.txt` to inspect a deliberately chosen historical log file. Input is bounded to 32 MiB, read only, and symlinks are rejected. The JSON report does **not** reproduce arbitrary log lines, game paths, user names, or other log details. It reports Title ID, shortened main Build ID, original hex length, per-log launch sequence number, archived-match status, and unassociated main-build count. If the log does not exist, an empty report is returned; an empty report is **not** evidence there are no games.

**Limits:** A historical main NSO Build ID is evidence of what Eden previously attempted to run, not proof of what is currently installed or proof that a cheat works. A single log may contain many games or old versions. Multiple builds for one Title ID are explicitly reported as ambiguous; contextless records are discarded. `current_installed_build_verified`, `compatible_cheats_verified`, and `installation_allowed` remain false. No ROMs, keys, saves, or emulator settings are accessed; no cheats are installed or activated. The staging/export process remains separate and opt-in.

### Compact inventory from Eden's registered folders

`cheatarium-eden-discover --summary --json` produces a compact view derived from **the same configured Eden roots**, existing cache labels and filenames as the full discovery report:

```sh
cargo run --locked -q -p cheatarium-client --bin cheatarium-eden-discover -- --db generated/v1 --summary --json
```

The summary identifies candidate base games by standard Title ID label shape, possible update containers using Eden's own `GetUpdateTitleID(base) = base | 0x800` rule, and likely DLC files grouped by user directory labels such as `dlc/`. It lists filenames with no embedded Title ID separately, rather than guessing their identities from adjacent files. It does not report each individual DLC Title ID as an independent playable game.

Every classification is explicitly **advisory**: an NSP filename may be wrong; a cached title may be stale; a directory labeled `dlc` does not verify an add-on; and a downloaded update in a game folder **does not mean that Eden has installed or enabled that update**. Likewise, neither `qt-config.ini` nor the game-list cache records the currently loaded executable Build ID. The summary doesn't open game containers or touch Eden, and it leaves all install/build compatibility flags false. The older full JSON format is unchanged unless the `--summary` flag is specified.
