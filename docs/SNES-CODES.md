# SNES cheat-code decoding

Cheatarium provides pure Rust decoding in `crates/cheatarium-codecs`; it does **not** execute cheats or modify emulator repositories. The v1 SNES index includes a derived `snes_decode` object for complete codes with either an explicit device format in the source filename or a strictly recognizable, homogeneous code syntax. The latter are **syntax candidates**, not source-confirmed device identities.

## Decoded representation

Source code strings remain unchanged in every entry's `code` field. When decoding fully succeeds, `snes_decode` adds:

```json
{
  "format": "game-genie",
  "address_space": "snes-cpu-bus-24-bit",
  "compatibility": "unverified-cartridge-build",
  "interpretation_basis": "declared-file-format",
  "writes": [{ "address_hex": "009E25", "value_hex": "00" }]
}
```

This is what the code *decodes to*, not proof the bus location is RAM, that the targeted game/region/revision is compatible, or that any emulator supports patching that address. A Game Genie ROM read substitution and a memory-write engine do not have identical semantics; a consumer must implement its own device-specific timing and mapping.

## Evidence-backed source composition

The additive, separately audited registry `interpretations/v1/snes.json` describes 27 independently documented original `+` entries from Donkey Kong Country. The corresponding read-only distribution artifact is `generated/v1/interpretations/snes.json`. Each reviewed record specifies the original source record, code ordinal, upstream file blob SHA, unchanged raw code string, version-alternative partitions and external citations.

For example, the source string `C2C9-4E2C+C2C1-4A9C` actually represents **two one-code alternatives for different game versions**, not one two-part program. Another example, `DBC1-3D6D+DCC1-34AD+DBC9-340D+DCC1-3D6D`, represents **two versions of a two-code combination**. The original [Game Genie Donkey Kong Country code table](https://gamegenie.com/cheats/gamegenie/snes/donkeykongcountry.html) explicitly separates the alternatives by version. It does **not** identify those ROMs by SHA-256.

```fish
cargo run --release -p cheatarium-client --bin cheatarium-query -- compositions --db generated/v1 --platform snes --game-key donkey-kong-country --relation revision-alternatives --json
```

The query returns the original source provenance and both the reviewed or unresolved grouping status. Both `rom_match_verified` and `safe_to_combine_or_auto_apply` remain false. Other source `+` records are marked `unresolved` until reviewed, even if their individual code bytes decode successfully.

The indexer and independent Python audit reject altered source blobs, missing ordinals, lost/reordered components, malformed evidence and mistakenly emitted simultaneous writes for revision alternatives. The composition auditor also re-reads each referenced original `.cht`, recalculates the Git-blob SHA-1 from its bytes and checks the exact original ordinal/code string. The release manifest separately reports the number of original files and ordinals checked. The generated registry and source bundle are checksummed in the published distribution; none of these integrity checks demonstrate gameplay compatibility.

### Unresolved-source review priority

The reproducible [SNES composition review queue](../generated/v1/reports/snes-composition-review-queue.json) groups still-unresolved `+` source occurrences by **advisory filename-derived game key**, sorted by descending unresolved source count, then game key. It publishes the top 100 candidate buckets with original source IDs, code ordinals, source blob hashes, region/declared-device hints and up to three raw examples per bucket.

This is an evidence-acquisition priority list, **not** a claim that the highest-ranked game has the most broken cheats, nor that `+` means simultaneous writes. It can be regenerated and independently checked by `tools/build_snes_review_queue.py`, with synthetic adversarial tests in CI.

The queue additionally reports `historical_publication_witnesses` and `occurrences_without_publication_witness` per filename candidate, plus corresponding archive-wide totals. These refer only to exact, source-audited external **textual** witnesses. They are not confirmed simultaneous programs, compatible builds, deduplicated cheats, or verified effects. Ranking remains based on original unresolved source occurrences, not the witness count.

### Inspect full source dossiers

The top-100 queue carries only three original examples per candidate. For an exhaustive, **read-only** inspection of one candidate, query the pinned SNES source bundle directly:

```sh
python3 tools/inspect_snes_joins.py --game-key push-over --limit 50
python3 tools/inspect_snes_joins.py --game-key push-over --offset 50 --limit 50
python3 tools/inspect_snes_joins.py --game-key push-over --witnessed-only
python3 tools/inspect_snes_joins.py --game-key push-over --unwitnessed-only --limit 50
```

The JSON dossier gives the total unresolved source-occurrence count, distinct **literal** code strings, contributing source-record counts, and a histogram of plus-separated text segment lengths. Each paginated occurrence preserves its original description, exact code text, original ordinal, title/region/device hints, source-enabled flag, verification label and upstream provenance, including Git blob SHA. Use `--source-record-id '...'` to examine just one original source; `--witnessed-only` and `--unwitnessed-only` are mutually exclusive research filters. The output keeps the pre-filter `total_unresolved_source_occurrences` and adds `selected_source_occurrences` for filtered pagination. Page size is capped at 500, and ordering is stable by source record ID and original ordinal.

The dossier, historical-publication validator, and review-queue builder reject duplicate original source IDs or source ordinals rather than double-counting or choosing an ambiguous record. Identical text strings are not automatically equivalent cheats. A text segment is **not** a verified device write or a recommended code partition. Historical code relationships, game identity, ROM compatibility and activation remain unverified. This inspection tool does not modify indexes, fetch ROMs, decode joined programs, or execute cheats.

### Near-identical source text families

Reviewers can reduce repetitive research by grouping unresolved source entries
that have the same number of literal `+`-separated parts and share **all but
one** text component within the **same original source file**:

```sh
python3 tools/analyze_snes_patterns.py --game-key push-over --min-members 3 --limit 25
python3 tools/analyze_snes_patterns.py --game-key lemmings --limit 25
```

The report ranks candidate text patterns by the number of original occurrences,
retains source ordinals, exact source strings, descriptions and upstream Git
blob, and identifies which one component text varies. It also counts how many
members have a separately validated historical publication witness. The
`--source-record-id` filter restricts one original file; `--offset` and
`--limit` paginate results.

Patterns are **source-text similarities only**: whitespace is trimmed for
component comparison without changing published original strings. Families
can overlap, and their counts must not be summed into unique cheats. A shared
prefix/suffix does **not** prove same effect, simultaneous writes, alternate
revisions, safe activation or ROM compatibility. The tool is offline,
read-only and covered by synthetic adversarial tests.

### Historical multi-part publication witnesses

A separately validated, source-bound [publication witness registry](../interpretations/v1/snes-published-groups.json) records 17 original unresolved SNES `+` strings whose exact multi-part code text also appears in historically published code listings: three each from **Ka-Blooey**, **Push-Over**, **Lemmings**, **Ogre Battle** and **Faceball 2000**, plus two from **Bahamut Lagoon**. Each claim includes the original source record/ordinal, upstream Git blob, unchanged joined code, publication URL and listing entry, and a transcription of the publication's effect label.

The external listings document how those strings were **published as a grouped entry**. They do not establish whether all components work in gameplay, whether the listing copied the source archive, which cartridge revision is required, or whether applying a decoded write sequence is safe. Accordingly, these records remain `composition.relation: "unresolved"` in the source index; all `execution_observed`, `rom_match_verified` and `safe_to_auto_apply` values are false. Publication witnesses are not counted among the 27 documented revision-alternative partitions.

The registry is independently validated against the pinned `snes.json.gz` source bundle, including unchanged code text, exact original ordinal and upstream blob. A second independent audit reads each underlying archived `.cht` file, recalculates its original Git blob SHA-1 from the **unaltered bytes**, checks that its archived path preserves the recorded upstream path and source ID (rejecting even byte-identical aliases), and confirms each cited original cheat ordinal and code text. Only then is the evidence published as [checksummed distribution data](../generated/v1/interpretations/snes-published-groups.json). Run `python3 tools/validate_snes_publications.py --check` after generating distribution snapshots. Synthetic tests reject tampered archives, unsafe paths, duplicate original cheat ordinals, altered source citations, component reordering and invented execution claims. These checks establish archival integrity, **not** gameplay verification or the independence of external websites.

External listing references: [Ka-Blooey](https://gamegenie.com/cheats/gamegenie/snes/kablooey.html), [Push-Over](https://gamegenie.com/cheats/gamegenie/snes/pushover.html), [Lemmings](https://gamegenie.com/cheats/gamegenie/snes/lemmings.html), [Ogre Battle](https://almarsguides.com/retro/walkthroughs/snes/games/ogrebattle/gamegenie/), [Faceball 2000](https://etherealgames.com/snes/f/faceball-2000/game-genie-codes/), and [Bahamut Lagoon](https://gamefaqs.gamespot.com/snes/563516-bahamut-lagoon/faqs/6647) (one entry additionally sourced from [Ethereal Games](https://etherealgames.com/snes/b/bahamut-lagoon/game-genie-codes/)). The dossier inspector includes a `historical_publication_witnesses` count and attaches the validated citation to each matching source occurrence; all execution and compatibility flags remain false.
## Supported formats

- **Game Genie:** one 4+4-character hyphenated SNES code per component (e.g. `DDB4-6F07`); the known Game Genie alphabet and 24-bit address-bit permutation are applied strictly.
- **Pro Action Replay / Action Replay:** exactly eight hexadecimal characters representing a 24-bit SNES CPU-bus address followed by one byte (e.g. `7E1E6B14`).
- **Source `+` joins:** a plus character in an imported code is **not evidence that all parts are meant to run simultaneously**. For unreviewed joins the original entry exposes `composition.relation: "unresolved"`. The pure codec still decodes syntactically valid components, but its `writes` vector is only a parse, **never a runnable compound program**.
- **Reviewed revision alternatives:** externally documented `+` source entries expose `composition.relation: "revision-alternatives"` with ordered `alternatives` (each alternative contains one or more components) and attributed evidence. They have **no `snes_decode` object for the joined string**. Consumers must neither concatenate their alternatives nor assume a ROM version from the filename; no verified ROM hash mapping exists.
- **Syntax decoder limits:** any component failing syntax validation leaves the entire input undecoded. Pure decoding accepts up to 64 components and 1024 source bytes, but cannot establish whether a source code joins alternative revisions or simultaneous device writes.
- **Wildcards and placeholders:** e.g. `7FC136XX` are preserved as source strings, but are deliberately not converted into concrete writes.
- **Unlabeled source files:** a full set of Game Genie-shaped components may be interpreted as Game Genie with `interpretation_basis: "code-syntax"`. A full set of eight-digit hexadecimal components may be interpreted as `format: "raw-snes-address-value"`, without falsely asserting Pro Action Replay provenance. Both have unverified compatibility.
- **Conflicting or ambiguous sources:** a declared format takes precedence. A failed declared-format decode stays undecoded, even if the code text could fit a different format. Mixed, placeholder, or unknown code syntax stays undecoded. Codes for other consoles are unaffected.

## Reference vectors

| Source text | Decoded SNES bus address | Byte |
| --- | --- | --- |
| `DDB4-6F07` | `009E25` | `00` |
| `D6B4-6F07` | `009E25` | `08` |
| `C222-D4DD` | `00D0D8` | `AD` |
| `7E1E6B14` | `7E1E6B` | `14` |

The decoder follows the documented SNES code encoding used by [Snes9x](https://github.com/snes9xgit/snes9x/blob/master/cheats2.cpp) and cross-checked against [bsnes-derived decoding](https://github.com/OpenEmu/BSNES-Core/blob/master/program.mm). The Super Mario World Game Genie sample codes also appear in historical published codebooks; that confirms example code text, not execution on arbitrary cartridges.

## Standalone Cheatarium tools

The decoder can be exercised without an emulator:

```sh
cargo run --release -p cheatarium-codecs --bin cheatarium-decode -- decode snes game-genie 'DDB4-6F07'
cargo run --release -p cheatarium-codecs --bin cheatarium-decode -- decode snes action-replay '7E1E6B14+7F80CAFF'
cargo run --release -p cheatarium-codecs --bin cheatarium-decode -- decode snes syntax '7E1E6B14+7F80CAFF'
python3 tools/audit_snes_codecs.py --json
```

The decoder prints versioned JSON containing the original code, decoded writes, evidence basis, and explicit `executable: false` and `rom_compatible: false` flags. It rejects malformed codes with a nonzero exit status. The audit partitions every SNES code field by decoded, malformed/placeholder, other format, or unknown source format.

## Coverage report from the pinned Libretro snapshot

The generated, per-snapshot [SNES codec audit](../generated/v1/reports/snes-codec-coverage.json) tracks precisely what is and is not interpreted. On the current 23,382-file Libretro import (SNES: 2,773 source files):

| Classification | Source entries |
| --- | ---: |
| Encoded SNES code fields | 68,094 |
| Successfully interpreted from explicitly named device format | See reproducible audit |
| Successfully interpreted from distinctive, unlabeled code syntax | See reproducible audit |
| Documented, version-alternative source groups (never emitted as combined decoded writes) | 27 |
| Unknown `+` grouping semantics | See reproducible audit |
| Incomplete or unrecognized code formats | See reproducible audit |

Syntax-only interpretations distinguish recognizable Game Genie code text from anonymous eight-hex-digit address/value candidates. The latter are **not** presented as proven Pro Action Replay cheats. Exact current counts, including revision alternatives and unresolved `+` groups, come from the generated audit. The complete audit is machine-readable, generated during CI, cross-checked against SNES source records, and checksummed in `distribution.json`.

Interpreted entries are not unique cheats and have **not** been verified against actual ROMs. Do not use these numbers to imply functional compatibility.

## Tests and release integrity

Rust unit tests cover valid/invalid/compound codes, lowercase inputs, wildcards, and bounded input. GitHub Actions also checks actual imported Super Mario World and 3 Ninjas cheat records, ensures a wildcard remains undecoded, and runs the distribution integrity checker across **all** consoles.

`decoded_snes_code_fields` in `catalog.json` and `distribution.json` counts fully decoded **source cheat entries**, not individual bus writes, unique cheats, compatible ROMs, or cheats tested during gameplay. Unmatched entries retain their original data and remain available for future curation. There is no implicit activation.
