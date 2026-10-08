# Design and boundaries

Cheatarium is a **game-cheat archive**, not an emulator and not a ROM collection. Organize around *what a player wants to do* while preserving *exactly what a source said*. The collection should be delightful to browse without destroying technical precision.

## Two-layer collection

- `archive/<source>/<upstream-path>`: unmodified native source files, retaining original relative paths and filenames. Imported files are **never silently edited**.
- `curated/<platform-id>/<game-id>/cheats.json`: human-oriented cheat entries. One effect may offer multiple formats/devices and edition-specific variants. Curated entries always cite a source.
- `sources/<source-id>.json`: machine-readable source identity, pinned upstream revision, reuse terms and an inventory of imported files.
- `platforms/platforms.json`: stable identifiers for console and handheld hardware. Directories only need to exist when there is content.
- `schemas/game.schema.json`: minimal contract for curated data. Archive native files do **not** have to fit it.

## Identity and compatibility

Platform → game → edition/region/revision → cheat effect → code variant.

A game is not the same as its release. A Game Genie code for a US SNES cartridge is not necessarily valid for a Japanese revision, an SNES Classic release, or another emulator. Keep separate records for variants when compatibility differs. If a ROM hash, serial, emulator core, memory domain, or device requirement is known, record it; if unknown, leave it unknown instead of guessing.

The `format` field names the *encoding or execution environment* (e.g., `snes-game-genie`, `snes-action-replay`). The `kind` field describes the *player-facing mechanism* (device-code, emulator-memory, button-sequence, password, unlockable, glitch, patch). Do not "convert" Game Genie to raw RAM writes by string substitution. Conversion needs platform-specific semantics.

A curated cheat can carry notes about side effects, game progress, permanence, conflicts, and a tested build. Its status starts `unverified`; only a real test should upgrade it to `verified`. No code should be enabled by default.

## Scope

**Now:** consoles and handhelds, especially offline/single-player use. Include Game Genie, GameShark, Action Replay, Pro Action Replay, CodeBreaker, native emulator formats, passwords, debug menus, button sequences, secrets, and glitches.

**Later:** arcade/computer platforms, search UI, format conversion, executable patches (only where redistribution is permitted), Rust-powered tools, and emulator integrations such as Starbyte.

**Never in this repository:** copyrighted ROM dumps, proprietary BIOS/firmware, unlock keys for paid content, account compromises, or multiplayer cheating tools.

## Import policy

1. Inspect the upstream archive, its license and any embedded attributions.
2. Pin the exact source revision, and record original path, destination path, and blob SHA.
3. Preserve original file bytes and filenames. Changes go in `curated/`, not `archive/`.
4. Do not claim a cheat works merely because it was present in an upstream dataset.
5. Dedupe equivalent records when curating, but retain original archive copies for reproducibility.
6. Preserve required license notices and maintain attribution. Link-only is the default where redistribution rights are unclear.

Cheatarium is independent from Sourcearium (textual corpora) and Observatorium (observational/event data): the native console-code payloads and their game-specific compatibility are essential to this project's purpose.
