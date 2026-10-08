# Adding cheats

This guide supports both giant dumps and a single code someone discovered while playing.

## 1. Choose a platform and game

Find the stable ID in `platforms/platforms.json`. Use `curated/<platform-id>/<game-slug>/cheats.json` for new curated records. Give different games different slugs even if their names overlap. Specify edition or region when known.

## 2. Import an upstream collection

- Confirm the exact upstream URL, revision and applicable redistribution terms. Do not assume 'public on GitHub' means reusable.
- Record a source in `sources/<source-id>.json`; add imported files to its inventory.
- Copy source files verbatim to `archive/<source-id>/<upstream-relative-path>`.
- Record upstream Git blob SHA per file where possible. This makes provenance and exact integrity checks possible.
- If redistribution rights are unresolved, document the source in `sources/README.md`, but do not vendor the dataset.

Never flatten every source into a universal filename convention. Native folder structures can have semantic meaning.

## 3. Curate an individual cheat

Add an effect-level record containing an `id`, a clear `name`, `kind`, and one or more `variants`. Each variant must have:
- A matching `edition` identifier, a format, its exact `code`, and a `source` pointing to the archived file.
- `status`: `unverified` for collected codes, `verified` only when actually tested, or `broken` when tested and failing.
- Optional caveats: emulator/core, region or revision, observed behavior, side effects, dependencies, author, date tested, or conflicts.

For passwords/button sequences/unlockables, store the literal instructions in `code` until richer per-kind conventions are introduced. Preserve exact characters, especially `0` versus `O` and hexadecimal values.

## 4. Validate

Run `python tools/validate.py` (or `uv run python tools/validate.py`). It checks manifest JSON, source-file integrity using Git blob SHA, curated references, edition/cheat ID uniqueness, and code values. It does not prove that cheats function in games.

## 5. Enrich over time

Record screenshots or actual test observations separately; prioritize adding known regional differences, multiple code devices, prerequisites, warnings, and cross-format equivalents. Do not overwrite unverified upstream evidence with guesses.
