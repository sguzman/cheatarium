# Desktop explorer

Cheatarium includes an optional native Linux browser for inspecting its offline
console cheat archive. The first release focuses on fast title discovery and
traceable original records, not editing or applying cheats.

## Launch

From the repository root, with a complete generated index:

```fish
cargo run --release -p cheatarium-client --features desktop --bin cheatarium-explorer -- --db generated/v1
```

The Rust/egui desktop dependency is optional. Normal CLI and library builds
do not require a GUI runtime. The app supports Wayland (including Hyprland)
and X11, with a lightweight OpenGL renderer. It never connects to an account
or requests ROM data.

## Browse workflow

Use the left pane to search filename-derived candidate titles across all
indexed consoles, or select a console. Choose a game to load its original
source files and cheat entries. This step checks the selected platform's
SHA-256 distribution manifest before displaying entries; the initial global
title catalog is also checksum-checked.

The central pane lists imported entries with original ordinals and source
filenames, and filters by original description, source file or entry type.
The right pane shows the selected entry's complete raw code text when present,
native fields, source-enabled flag, import verification status, source revision,
path, license and Git blob. The **Copy original text** control copies only
the original string; it does not load an emulator or apply a cheat.

Different source entries remain separate even when they have identical code
text. The entry's identity is its exact original source record ID and ordinal.
Headings and native-memory metadata are preserved rather than being rewritten
as device codes.

## Integrity and limits

- Title and game groupings are suggestions inferred from source filenames,
  **not verified ROM identity**.
- Descriptions reflect archived authors' wording; they are not verified
  statements of what the codes do.
- An upstream `source_enabled` flag is historical source metadata, not
  permission or an instruction to activate the code.
- Local SHA-256 checks verify files against the checked-in manifest. The
  manifest is not digitally signed or independently authenticated.
- The app is read-only: it does not change the archive, write game files,
  launch emulators, or transmit searches or codes.
- Loading a new game verifies source integrity on one background worker,
  leaving the desktop window interactive. A newer selection supersedes queued
  older requests; late results cannot replace the current selection. An already
  running source read still finishes before the next request starts.
- This remains an initial desktop MVP. Screenshot-based layout and live Wayland
  interaction QA remain outstanding even when headless compilation and unit
  tests pass.

The canonical terminal and programmatic interfaces are documented in
[Consumers](CONSUMERS.md). Raw sources, independently reviewed evidence,
and gameplay observations remain distinct data layers.
