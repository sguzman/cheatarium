# Upstream source anomalies

All 23,382 archived Libretro `.cht` files passed Git blob integrity checks. There are **five remaining count mismatches** in the pinned upstream snapshot.

| Platform | Original file | Declared entries | Parsed entries |
| --- | --- | ---: | ---: |
| TurboGrafx-16 | Bloody Wolf (USA).cht | 4 | 3 |
| TurboGrafx-16 | Genji Tsuushin Agedama (Japan).cht | 2 | 1 |
| TurboGrafx-16 | Genpei Toumaden (Japan).cht | 2 | 1 |
| TurboGrafx-CD | Kabuki Ittouryoudan (Japan) (FABT).cht | 11 | 3 |
| Sega 32X | WWF WrestleMania - The Arcade Game (USA).cht | 8 | 9 |

Earlier diagnostics counted only `cheatN_code` fields, incorrectly excluding valid Nintendo DS section headings and address/value-based memory cheats. The validator now counts all indexed `cheatN_*` entries, and the Rust exporter preserves all additional memory fields.

An anomaly does not prove cheats are broken. Preserve originals under `archive/` and document any corrections as derived records with source provenance.

Libretro commit `fbeefcb46c2e1b20a7e2945f34a694a41b2d6f90`; audited 2026-10-08.
