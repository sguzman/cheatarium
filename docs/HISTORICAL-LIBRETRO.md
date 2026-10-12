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
to re-fetch the two complete, immutable Git cheat trees and compare the report;
`--write` regenerates it. The audit requires no ROMs, no emulator and no
cheat execution.

The dedicated historical audit workflow also publishes a downloadable,
metadata-only `historic-libretro-unarchived-metadata` artifact listing each
of the 4,369 byte-absent occurrences by pinned original path, Git blob SHA-1
and byte size. Each row links to the upstream immutable source revision.
`tools/export_historic_libretro.py --output <file>` recreates the inventory
from Git metadata; it rejects a source-count mismatch against the committed
audit. Filename-only `(RetroArch Rumbles)` signals are reported separately,
but no code or rumble payload is downloaded, imported, or gameplay-verified.
The artifact is not part of the archived console-cheat totals.

## Interpretation

The historical snapshot contains 15,714 native `.cht` file occurrences,
versus 27,915 in the modern baseline. An exact byte comparison finds
4,779 historical occurrences with no matching blob in the baseline.
Of those, 410 have a matching blob in Cheatarium's already archived
BitHeX delta. A second exact-byte comparison against **all 81 archived
source collections** found no further matches. The remaining **4,369
file occurrences (4,057 distinct Git blobs)** have no exact byte match
in Cheatarium's current 67,834-file archive.
Most of these belong to Nintendo DS (3,213 occurrences), followed by
Game Boy Advance (419), ZX Spectrum +3 (393) and MSX-family (317).

The archive-wide cross-check uses the original Git blob hashes
recorded in every pinned source manifest. It shows **byte absence**,
not semantic novelty. Some paths may be renames, removed versions or duplicated bytes;
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

The cross-check against the entire current archive is complete; no
additional exact blob matches were found outside the baseline and BitHeX
delta. Next acquisition step: separate rumble/supporting files from
cheat data and investigate original-author provenance for the remaining
historical bytes. Preserve unmodified originals and source attribution
only when redistribution rights can be defended.


## Other link-only source overlap

The historical workflow also produces a metadata-only
`historic-vs-external-blobs.json` report. It compares each of the
unarchived historical Git blobs with all pinned external Git-blob
references, excluding the exact bytes already preserved in the archive.
It records source/folder attribution and bounded matching examples, and
calculates the distinct byte-gap union **without adding mirror duplicates**.

A shared Git blob is byte identity only. The union is not a count of
distinct games, functioning codes, or legally redistributable cheat bodies.
This audit deliberately does not copy any historical or external payload.
The combined report is published alongside the historical filename
inventory as a CI artifact, not inserted into archived-cheat bundles.


## Additional historical RetroArch mirror audit (2026-10-12)

Two further pinned GitHub forks have been checked by **Git blob SHA-1**, not assumed game identity. The [research mirror-lead inventory](../research/retroarch-mirror-source-gaps-v1.json) pins original source paths, Git blobs, sizes, repository commits and original source folders. `gcmcom/libretro-database` has 10,872 `.cht` file occurrences, of which **10,861 exactly match byte hashes in the pinned 2019 Emupedia fork**. Eleven occurrences do not match any of the three compared inventories: archived Libretro baseline, Emupedia, or archived BitHeX delta. A second old RetroArch cheats fork, `Smoker1/RetroArch-174-and-above-Cheats`, has 518 `.cht` files, of which 75 (66 distinct Git blobs) are absent from those three compared byte inventories. Across the two sources this is **86 occurrences, 77 distinct candidate blobs**. Smoker1 also has 98 separate `.ini` files, **not included in this `.cht`-specific audit**. None of these counts is an independently verified game count or a comprehensive global archive novelty claim.

Both new forks lack verified original-author blanket redistribution grants; **no cheat bodies are archived or added to the external platform bundles by this research report**. The historical audit workflow now verifies the frozen source Git tree metadata online, independently recomputes the byte overlap, checks the committed metadata-only report, and publishes the standalone file `retroarch-mirror-gaps.json` in its CI artifact. All three Libretro-family comparison corpora remain pinned. A matching SHA records byte identity only, never playable compatibility or legal permission.
