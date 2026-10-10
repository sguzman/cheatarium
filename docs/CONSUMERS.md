# Cheatarium consumers

Cheatarium publishes **read-only**, revision-pinned, compressed JSON and a Rust client in `crates/cheatarium-client`. It is not an emulator and does not modify emulator repositories.

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

`discover` searches advisory game-title groups across **all 44 local platform catalogs** without first choosing a console. Its platform-qualified results include the original candidate key, alternate title hints, source-record references, region/format hints, and counts. Results are sorted by platform and candidate key, then paginated with `--offset` and `--limit`. Consumers can follow one result with `game --platform PLATFORM --game-key KEY` for checksum-checked original-source inspection. Discovery verifies the SHA-256 hashes and byte lengths of `catalog.json` and all 44 game-title / source-ID indexes against the local `distribution.json` manifest before searching. It does **not** read or hash the much larger cheat-entry bundles, authenticate the manifest's publisher, inspect ROMs, or infer cheat compatibility. Its JSON distinguishes verified title-index file checksums from the unauthenticated manifest. The follow-up `game` command verifies the selected platform's complete distribution before reading original cheat sources.

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
