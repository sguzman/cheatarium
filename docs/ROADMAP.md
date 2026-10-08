# Roadmap and acquisition queues

**North star:** a huge, pleasant, searchable collection, where original cheat files and meaningful game/edition/device relationships are both preserved.

## Collection pipeline

1. **Discover** a source; identify platforms, file formats, size, provenance, and licenses.
2. **Clear** reuse rights or mark the source link-only.
3. **Acquire** a pinned snapshot (start with bounded batches to audit the pipeline).
4. **Inventory** imported files and validate integrity.
5. **Normalize** into per-game curated records without discarding raw material.
6. **Verify** codes against actual game builds and supported emulators.
7. **Publish** searchable indexes, then integrations/exporters.

## Platform queue

- [x] Register console/handheld platform IDs.
- [x] Start Libretro NES and SNES proof-of-concept imports.
- [ ] Scale Libretro NES/SNES archives in batches with import tooling and detailed manifests.
- [ ] Expand Nintendo: Game Boy, Game Boy Color, Game Boy Advance, N64, DS, GameCube, Wii, 3DS.
- [ ] Expand Sega: Master System, Genesis/Mega Drive, Game Gear, Saturn, Dreamcast.
- [ ] Expand Sony: PlayStation, PS2, PSP, PS3, Vita, PS4.
- [ ] Evaluate Nintendo Switch title-ID/build-ID cheat datasets.
- [ ] Evaluate Microsoft Xbox families, Atari, NEC, SNK, Bandai, 3DO.
- [ ] Add link-only indexes for valuable sources whose redistribution rights cannot be confirmed.

## Infrastructure queue

- [x] Brief, purposeful README and design/ingestion documentation.
- [x] Source manifests with exact upstream paths/revisions/blob IDs.
- [x] Game/edition/format schema and integrity validator.
- [ ] First automatic upstream **bulk import** command with limits, filtering, and deterministic records.
- [ ] Generate source-independent game and platform indexes.
- [ ] Dedupe equivalent codes while retaining original provenance.
- [ ] Search by game, platform, cheat effect, author, source, and code format.
- [ ] Reliable console-specific converters/exporters.
- [ ] Optional Rust CLI and offline explorer.
- [ ] Starbyte integration, starting with SNES cheats when emulator capabilities support them.

Keep the README about the project; keep work tracking here.
