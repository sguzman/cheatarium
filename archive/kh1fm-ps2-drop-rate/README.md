# Kingdom Hearts Final Mix PS2 — Enemy Item Drop Rate

A runtime PNACH patch that applies a fixed **3× or 4× multiplier** to ordinary enemy item-drop rolls in *Kingdom Hearts Final Mix* for PlayStation 2.

The recommended version is **4×**. A **100× diagnostic build** is included only for quickly confirming that the hook works on a particular setup.

## Compatibility

- Game: *Kingdom Hearts Final Mix* (PS2)
- Serial: `SLPS-25198`
- Tested CRC: `BD3FB870`
- Tested game build: English-translation-patched Final Mix ISO
- Confirmed emulator/frontend: ARMSX2
- Format: standard PCSX2-style PNACH

The patch has not been verified against every English translation revision, the untouched Japanese executable, or every PCSX2-derived emulator. A different CRC may indicate that the executable addresses are different; do not assume compatibility solely because the game boots.

## What it changes

The patch replaces the runtime value used as the game's global Lucky Strike/item-drop multiplier with a fixed value, then prevents the normal Lucky Strike addition from changing it.

- It multiplies the probability of ordinary enemy item-rolls; it does **not** create four copies of an item after a successful roll.
- It affects ordinary enemy drops such as consumables and synthesis materials.
- It does not intentionally change treasure chests, scripted rewards, guaranteed rewards, or save-file structures.
- High-probability rolls may become effectively guaranteed, but an enemy will not necessarily drop every possible item after every defeat.

### Lucky Strike

Lucky Strike does **not stack** with this patch. While the patch is enabled, equipping Lucky Strike provides no additional drop-rate benefit, so you should normally unequip it and spend the AP elsewhere.

## Installation

1. Choose either the `4x` or `3x` directory under [`pnach`](pnach). The 4× version is recommended.
2. Import `SLPS-25198_BD3FB870.pnach` through your emulator's per-game patch interface, or place it in the appropriate patch directory.
3. If you already have a PNACH with the same filename, merge the complete bracketed group into your existing file instead of overwriting your other patches.
4. Enable the emulator/frontend's master **Patches** setting and enable the individual drop-rate group.
5. Fully restart or reset the game after installing or changing variants.

Only install or enable **one multiplier variant at a time**. If two variants write the same addresses every frame, the effective result depends on processing order.

The `100x-test` file is deliberately excessive and is intended only as a diagnostic. If it is working, normal enemies should produce an unmistakable shower of eligible drops within a few battles. Return to 3× or 4× afterward.

## Existing PNACH files and other mods

The injected routine occupies `0x000FF000–0x000FF010` and also patches `0x00125378` and `0x00125520`. Another patch writing any of those addresses will conflict.

An earlier development build used `0x000FD0A0–0x000FD0B0`. Do not use that build: the location overlaps the full Critical Mix `NTSC Controls + Analog Camera` routine. The public version was relocated and re-tested at 100× specifically to avoid that collision.

The patch does not include or redistribute the English translation, Critical Mix, a 60 FPS patch, control patches, game executable, ISO, or BIOS.

## Testing performed

- A 100× build at the original hook confirmed that the final approach affected both ordinary consumable and synthesis-material rolls.
- The routine was relocated to `0x000FF000`; the relocated 100× diagnostic was then independently confirmed on ARMSX2.
- The same implementation at 4× produced a noticeable but substantially less extreme increase during normal play.

Testing was performed on one translated game build and is not a complete playthrough certification.

## Why this approach

Several approaches were considered or tested:

- Writing `100.0` directly to a suspected live value at `0x00306848` produced misleading partial results: generic potion drops increased, but expected synthesis drops such as Lucid Shards from Shadows did not. That approach was discarded.
- Changing the vanilla `0.5` Lucky Strike coefficient could make each equipped copy stronger, but it would remain dependent on party abilities rather than provide the requested fixed multiplier.
- The first working injected routine used a code cave later found to overlap a known analog-camera patch. The routine was moved to `0x000FF000` and the move was validated with the conspicuous 100× test.

See [TECHNICAL.md](TECHNICAL.md) for the hook and instruction-level explanation.

## Support and disclaimer

This is an unsupported hobby patch. It is provided as-is, with no promise of maintenance, compatibility updates, troubleshooting, or fixes. It may conflict with other patches or behave unexpectedly in untested game revisions or situations.

Although the patch does not intentionally modify save structures, keep ordinary backups of saves you care about. Inventory obtained from enemy drops is, of course, saved normally. Use the patch at your own risk.

## Credits

- Reverse engineering, patch design, and documentation: **OpenAI Codex**
- Testing and validation on ARMSX2: **PreachingPython**

## License

Released under the permissive [MIT License](LICENSE).
