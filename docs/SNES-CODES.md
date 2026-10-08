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

## Supported formats

- **Game Genie:** one 4+4-character hyphenated SNES code per component (e.g. `DDB4-6F07`); the known Game Genie alphabet and 24-bit address-bit permutation are applied strictly.
- **Pro Action Replay / Action Replay:** exactly eight hexadecimal characters representing a 24-bit SNES CPU-bus address followed by one byte (e.g. `7E1E6B14`).
- **Compound groups:** separated by `+`; components stay ordered and linked as a single original cheat. If *any* component fails, the group stays undecoded. Up to 64 components and 1024 input bytes are accepted.
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
| Successfully interpreted from explicitly named device format | 14,750 |
| Successfully interpreted from distinctive, unlabeled code syntax | 51,852 |
| **Total interpreted** | **66,602** |
| Labeled device codes with malformed/incomplete/placeholder data | 1,478 |
| Remaining code fields without recognized syntax or format | 14 |

The syntax-only interpretations include **19,380 recognizable Game Genie entries** and **32,472 anonymous eight-hex-digit address/value candidates**. The latter are **not** presented as proven Pro Action Replay cheats. The complete audit is machine-readable, generated during CI, cross-checked against SNES source records, and checksummed in `distribution.json`.

Interpreted entries are not unique cheats and have **not** been verified against actual ROMs. Do not use these numbers to imply functional compatibility.

## Tests and release integrity

Rust unit tests cover valid/invalid/compound codes, lowercase inputs, wildcards, and bounded input. GitHub Actions also checks actual imported Super Mario World and 3 Ninjas cheat records, ensures a wildcard remains undecoded, and runs the distribution integrity checker across **all** consoles.

`decoded_snes_code_fields` in `catalog.json` and `distribution.json` counts fully decoded **source cheat entries**, not individual bus writes, unique cheats, compatible ROMs, or cheats tested during gameplay. Unmatched entries retain their original data and remain available for future curation. There is no implicit activation.
