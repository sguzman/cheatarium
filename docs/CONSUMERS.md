# Consuming Cheatarium from Rust emulators

Cheatarium now includes a **read-only Rust client** in \`crates/cheatarium-client\`. It reads the checked-in \`generated/v1/catalog.json\` and per-console compressed \`*.json.gz\` bundles. It never reads ROMs, touches emulator memory, downloads anything, or enables cheats.

## Try the bundled search tool

\`\`\`sh
cargo run --release -p cheatarium-client --bin cheatarium-query -- search \
  --db generated/v1 --platform snes --title "Super Mario World"
\`\`\`

Add \`--json\` to receive source records, their codes, and provenance as machine-readable output.

## Rust consumer API

The crate exposes \`load_catalog(dir)\`, \`load_platform(dir, "snes")\`, \`Bundle::search_title("Mario")\`, \`Bundle::by_candidate_game_key("super-mario-world")\`, and \`Code::is_code()\`.

A Starbyte adapter should:

1. Open a user-supplied or explicitly configured local \`snes.json.gz\` snapshot through this library.
2. Show title candidates and source variants in the SNES game UI or CLI.
3. Explicitly confirm the loaded cartridge's region/revision; use reliable hash/serial matching once that data is available.
4. Support an opt-in execution engine that understands the actual SNES cheat format (Game Genie/Action Replay, memory domain and timing). Not every indexed string is a valid direct memory write.
5. Never automatically activate imported codes, and never treat a section heading as executable.

**Current boundary:** the client and CLI can read, inspect, and search. Starbyte itself is not yet wired to this client, and no cheat-execution engine is claimed. The initial catalog is transport-ready, not runtime-code-ready.

Consumers should pin the exact Cheatarium revision or a future published version; do not depend on mutable branch HEAD or unverified title-only automatic matching. Avoid adding Cheatarium source archives as a build dependency: only ship the needed \`snes.json.gz\` and \`catalog.json\` files.
