# Klonoa 2: Lunatea's Veil - Hard Mode (v1.6)

A difficulty mod for the USA version of Klonoa 2 on PS2 (`SLUS_201.51`, NTSC-U). It makes the game harder in a few focused ways: less health, fewer lives, less healing, and faster enemies and bosses. Nothing else is touched. All changes were play-tested.

You need your own copy of the original NTSC-U ISO.

## What changes

| Change | Original | Hard Mode | Notes |
|---|---|---|---|
| Maximum HP | 3 HP | 2 HP | Every hit matters more, leaving less room for mistakes. |
| Small Heart | Guaranteed heal | 20% chance to heal | If collected at 1 HP and the heal roll fails, a warning sound cue plays. When a roll succeeds, the normal pickup sound plays. |
| Double Heart / Large Heart | Guaranteed heal | Guaranteed heal | Left unchanged because it is a rare item deliberately placed by the level designers. |
| Starting Lives | 3 | 2 | Fewer attempts available from the start. |
| Maximum Lives | 99 | 4 | Because the game still lets you play at 0 lives, this means up to 5 lives total. |
| Stage 1UPs | Always grants 1 life | 50% chance to give you 1 life | 1UPs earned from collecting 100 Dream Stones are still guaranteed. |
| Boss Comeback 1UP and 1HP | May appear when the player is struggling | Removed | Bosses no longer drop a life capsule or small heart as an emergency comeback mechanic. |
| Stage Enemies | Normal speed | 1.5x faster | Movement, projectiles, and animations are all accelerated. |
| Bosses | Normal speed | 1.5x overall | Folgaran (first boss): 2.75x. Leptio (second boss): 1.25x. Biskarsh (third boss): 1.5x. Polonte (fourth boss): vanilla speed. Leorina (fifth boss): 1.5x. King of Sorrow (final boss): 1.5x. |
| Tat & Capture Arenas | Normal speed | Unchanged | Kept at the original speed to preserve the intended arena mechanics. |

## How to install

Pick one of the two options.

### Option 1: patch the ISO (works on any emulator or a real console)

1. Download [Delta Patcher](https://github.com/marco-calautti/DeltaPatcher).
2. Select your original ISO, then `Klonoa2_HardMode.xdelta`, then choose the output file name.
3. Click Apply and play the new ISO.

Command line, if you prefer: `xdelta3 -d -s "Klonoa 2 - Lunatea's Veil (USA).iso" Klonoa2_HardMode.xdelta "Klonoa 2 [Hard Mode].iso"`

`CHECKSUMS.txt` has the SHA-256 hashes of the original ISO, the patch, and the result, so you can check that everything matches.

### Option 2: cheat file for PCSX2 (no ISO patching)

1. Copy `SLUS-20151_2F56CBC9.pnach` into the `cheats` folder of your PCSX2 install.
2. In PCSX2, open the game properties and turn on Enable Cheats.
3. Boot the original ISO and play.

## Notes

- Starting lives apply to a new save file. An existing save keeps the lives it already has.
- v1.6 fixes a bug where the cutscene after beating a boss played without any audio.

Curious about how it works, or want to tune the difficulty yourself? See `TECHNICAL.md`.
