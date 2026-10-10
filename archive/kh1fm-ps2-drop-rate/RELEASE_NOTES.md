# v1.0.0 — Initial release

Fixed enemy item-drop-rate multipliers for the English-patched PS2 release of *Kingdom Hearts Final Mix* (`SLPS-25198`, CRC `BD3FB870`).

## Included

- Recommended fixed 4× multiplier
- Optional fixed 3× multiplier
- 100× diagnostic variant for compatibility testing
- Installation, Lucky Strike, compatibility, and technical documentation

## Validation

The final routine was tested at 100× before and after relocation to `0x000FF000`. Eligible consumable and synthesis-material drops appeared en masse. The relocated 4× build uses the identical hook and routine with only the IEEE-754 multiplier constant changed.

## Important

- Lucky Strike does not stack with these fixed multipliers; unequip it and use the AP elsewhere.
- Install or enable only one multiplier variant.
- This release is unsupported and has not been validated through a complete playthrough.
- No game image, executable, translation, BIOS, or third-party patch is included.
