# Historical Libretro fork coverage (Emupedia, 2019)

Cheatarium's complete pinned Libretro import does **not** mean that it
preserves every historical version of those cheats. A Git-tree audit
found substantial exact-byte historical material that no longer matches
the pinned modern Libretro source.

The frozen [Emupedia fork](https://github.com/Emupedia/emupedia-docs-libretro-database)
is pinned at commit `c68abd0536adbb595bb833594de237e96626020d`
(August 17, 2019). The comparison uses only Git tree metadata and blob
SHA-1 identities; **no historical cheat payloads were imported**.

## Reproducible snapshot and method

- Emupedia historical cheat subtree: `f56cd8051f3643998a182493c7ea569f62ba7fd3`
- Modern Libretro baseline: `fbeefcb46c2e1b20a7e2945f34a694a41b2d6f90`
  (`cht/` tree `82732f21cd5b083ad7b8625fa7c8ca1ca920b6b1`)
- Already archived BitHeX fork delta: `837bc5b7b4b872f5b48c6b2e683819ec02db23b8`
- Executable auditor: [tools/audit_historic_libretro.py](../tools/audit_historic_libretro.py).
- Published [JSON report](../generated/v1/reports/libretro-historical-fork-coverage.json)
  includes strict reconciliation, bounded provenance samples and folder counts.

Run `python3 tools/audit_historic_libretro.py --offline-check` to
verify the committed summary without external requests. Run `--check`
to re-fetch the two immutable Git trees and compare the report; `--write`
regenerates it. The audit requires no ROMs, no emulator and no cheat execution.

## Interpretation

The historical snapshot contains 15,714 native `.cht` file occurrences,
versus 27,915 in the modern baseline. An exact byte comparison finds
4,779 historical occurrences with no matching blob in the baseline.
Of those, 410 have a matching blob in Cheatarium's already archived
BitHeX delta. The remaining **4,369 file occurrences (4,057 distinct
Git blobs)** have no exact byte match in those **two** archived datasets.
Most of these belong to Nintendo DS (3,213 occurrences), followed by
Game Boy Advance (419), ZX Spectrum +3 (393) and MSX-family (317).

These **are not yet proven absent from every Cheatarium source**;
the audit compares two pinned Libretro-derived source inventories,
not every one of Cheatarium's independent console collections.
Some paths may be renames, removed versions or duplicated bytes;
byte inequality does not establish a new game, new effect or valid
cheat. In particular, the snapshot includes RetroArch rumble files
alongside cheat-oriented `.cht` filenames.

## Rights and acquisition decision

The pinned Emupedia root tree contains no root `LICENSE` file.
The modern Libretro repository has a CC BY-SA 4.0 license, but a fork
and a host's project-level license do not independently establish
permission from every historical cheat author. Keep Emupedia as a
**historical metadata/provenance lead** pending individual-author and
upstream-rights review. Do not bulk-copy these code bodies yet.

Next acquisition step: identify which remaining historical blobs are
already present in other archived sources, separate rumble/supporting
files from cheat data, and find provenance for genuinely unpreserved
cheat bytes. Preserve unmodified originals and source attribution
only when redistribution rights can be defended.
