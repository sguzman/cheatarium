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

The indexer and independent Python audit reject altered source blobs, missing ordinals, lost/reordered components, malformed evidence and mistakenly emitted simultaneous writes for revision alternatives. The generated registry and source bundle are checksummed in the published distribution.

### Unresolved-source review priority

The reproducible [SNES composition review queue](../generated/v1/reports/snes-composition-review-queue.json) groups still-unresolved `+` source occurrences by **advisory filename-derived game key**, sorted by descending unresolved source count, then game key. It publishes the top 100 candidate buckets with original source IDs, code ordinals, source blob hashes, region/declared-device hints and up to three raw examples per bucket.

This is an evidence-acquisition priority list, **not** a claim that the highest-ranked game has the most broken cheats, nor that `+` means simultaneous writes. It can be regenerated and independently checked by `tools/build_snes_review_queue.py`, with synthetic adversarial tests in CI.

### Inspect full source dossiers

The top-100 queue carries only three original examples per candidate. For an exhaustive, **read-only** inspection of one candidate, query the pinned SNES source bundle directly:

```sh
python3 tools/inspect_snes_joins.py --game-key push-over --limit 50
python3 tools/inspect_snes_joins.py --game-key push-over --offset 50 --limit 50
```

The JSON dossier gives the total unresolved source-occurrence count, distinct **literal** code strings, contributing source-record counts, and a histogram of plus-separated text segment lengths. Each paginated occurrence preserves its original description, exact code text, original ordinal, title/region/device hints, source-enabled flag, verification label and upstream provenance, including Git blob SHA. Use `--source-record-id '...'` to examine just one original source; page size is capped at 500, and ordering is stable by source record ID and original ordinal.

Identical text strings are not automatically equivalent cheats. A text segment is **not** a verified device write or a recommended code partition. Historical code relationships, game identity, ROM compatibility and activation remain unverified. This inspection tool does not modify indexes, fetch ROMs, decode joined programs, or execute cheats.

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
