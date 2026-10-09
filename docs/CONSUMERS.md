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
cargo run --release -p cheatarium-client --bin cheatarium-query -- platforms --db generated/v1 --offset 0 --limit 36 --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- games --db generated/v1 --platform snes --title 'Chrono Trigger' --json
cargo run --release -p cheatarium-client --bin cheatarium-query -- game --db generated/v1 --platform snes --game-key push-over --offset 0 --limit 20 --json
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

`platforms` lists supported indexed consoles/handhelds without needing `--platform`, with source-file counts, filename-derived game-candidate counts, code/memory counts and artifact paths. Its `--offset`/`--limit` pagination supports small command palettes and GUIs. This is catalog enumeration, **not** proof of any game identity or imported cheat's functionality. A title-search workflow is `platforms` → `games --platform ... --title ...` → `game --game-key ...` → `source --source-record-id ...`.

`game` accepts one **exact advisory candidate game key** (`--game-key`) and joins the separately generated game index to its original per-platform source records. It returns full source IDs, filenames, source/region/format hints, Git blob and upstream licensing provenance, import warnings and per-source counts without dumping all cheat entries. Results use `--offset` and `--limit` with unpaginated totals. Missing/duplicate references, mismatched game keys, and disagreeing source-code/memory counts cause errors; the local distribution is checksum-checked first. The command never promotes a filename match to ROM identity or authorizes activation. Use the returned `source_record_id` with `source` to page individual original cheats.

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
