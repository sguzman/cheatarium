# Technical notes

## Summary

The final patch hooks the point where the game initializes a runtime item-drop multiplier, writes a selected IEEE-754 floating-point value, and suppresses the later vanilla Lucky Strike addition.

This targets the calculation that actually feeds ordinary enemy item-rolls rather than repeatedly writing a nearby global-looking value and assuming it is authoritative.

## Hook

At EE address `0x00125378`, the unmodified executable contains:

```asm
swc1 $f0, 0($v1)       # E4600000
```

The patch replaces it with:

```asm
jal 0x000FF000         # 0C03FC00
```

The original instruction at `0x0012537C` remains in the branch delay slot. Execution returns at `0x00125380`.

The injected 4× routine is:

```asm
lui  $at, 0x4080       # upper half of float 4.0
mtc1 $at, $f1
swc1 $f1, 0($v1)
jr   $ra
nop
```

The instruction at `0x00125520` is also replaced with `nop`. In the original routine it is the floating-point addition that applies the later Lucky Strike contribution; suppressing it keeps the selected multiplier fixed.

## Multiplier constants

Only the first instruction of the injected routine changes between variants:

| Variant | Float bits | Injected instruction |
|---|---:|---:|
| 3× | `0x40400000` | `3C014040` |
| 4× | `0x40800000` | `3C014080` |
| 100× diagnostic | `0x42C80000` | `3C0142C8` |

The 100× variant is a diagnostic signal, not a claim that every observed drop frequency will scale linearly by exactly 100. Individual rolls can saturate, be capped, or bypass this calculation.

## Why the code cave is at `0x000FF000`

The executable's first loadable segment begins at `0x00100000`, leaving `0x000FF000` outside the loaded game image. The five-instruction routine occupies `0x000FF000–0x000FF010`.

The first working development version used `0x000FD0A0–0x000FD0B0`. Inspection of the older Critical Mix PNACH showed that its full `NTSC Controls + Analog Camera` routine occupies `0x000FD050–0x000FD124`, creating a direct collision. The routine was therefore relocated, and the relocated location and jump were validated using the 100× build before the 4× release was prepared.

No code cave is universally collision-proof. Any other PNACH writing `0x000FF000–0x000FF010`, `0x00125378`, or `0x00125520` is incompatible unless the patches are manually combined.

## Discarded or alternative approaches

### Direct runtime-value write

An early test repeatedly wrote the float `100.0` to `0x00306848`. It appeared promising because ordinary enemies began dropping potions more frequently, but Shadows did not produce the expected Lucid Shards. This demonstrated that increased generic consumable drops alone were not sufficient evidence that the material drop-table calculation had been reached. The direct-write approach was discarded.

### Stronger Lucky Strike coefficient

The game multiplies a Lucky Strike contribution by `0.5` at `0x0012551C`. Removing that multiplication can produce progression-dependent results such as approximately 1×/2×/3×/4× with zero/one/two/three active Lucky Strike copies. That is potentially useful as a different mod, but it does not provide a fixed multiplier and was not selected for this release.

### Fixed initialization hook

Hooking the initialization store proved reliable in the conspicuous 100× test and remained effective at 4×. NOPing the subsequent addition prevents the game from changing the selected fixed value according to equipped Lucky Strike abilities.

## Evidence and limits

The following behavior was observed on the English-patched `SLPS-25198`, CRC `BD3FB870`, running in ARMSX2:

- 100× caused eligible enemy drops, including synthesis materials, to appear en masse.
- The relocated 100× implementation produced the same conspicuous result.
- 4× subjectively increased drops during ordinary play.

This is strong functional evidence for the hook but is not a large-sample statistical analysis of every enemy and drop-table entry. The entire game has not been regression-tested with the patch.
