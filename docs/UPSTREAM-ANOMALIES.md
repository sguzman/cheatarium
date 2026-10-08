# Upstream format anomalies

The first complete NES/SNES Libretro import passed **byte-level Git blob integrity** checks, but the original `.cht` data contains the following count inconsistencies. They are warnings, not changes to the archived material.

| Platform | Upstream file | Declared cheats | Parsed `cheatN_code` lines |
| --- | --- | ---: | ---: |
| NES | Chip 'n Dale - Rescue Rangers (USA, Europe) (Action Replay).cht | 1 | 0 |
| NES | Goonies 2 - Fratelli Saigo no Chousen (Japan) (Action Replay).cht | 1 | 0 |
| NES | Super Mario Bros. (World) (Action Replay).cht | 7 | 6 |
| NES | Super Mario Bros. 2 (USA, Europe) (Action Replay).cht | 1 | 0 |
| NES | Top Gun - The Second Mission (USA, Europe) (Action Replay).cht | 1 | 0 |
| SNES | Super Mario All-Stars + Super Mario World (USA, Europe) (Game Genie).cht | 14 | 16 |
| SNES | Super Mario World (USA) (Game Genie).cht | 149 | 150 |

These differences do not establish whether individual codes work; they flag potentially inconsistent upstream metadata. Never modify files under `archive/` to hide a source discrepancy. Corrections and interpretations belong in `curated/`, with provenance back to the originals.

Initial source: [Libretro Database](https://github.com/libretro/libretro-database) commit `fbeefcb46c2e1b20a7e2945f34a694a41b2d6f90`. Detected 2026-10-08.
