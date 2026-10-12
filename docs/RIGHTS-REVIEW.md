# Bulk cheat acquisition: pinned original-author rights review

The project currently archives 67,834 original files and separately indexes
42,533 **link-only** external occurrences. Not all are cheats, unique titles,
or independently verified modifications.

The [priority scorecard](../generated/v1/reports/acquisition-priorities.json)
ranks sources by exact Git blob bytes absent from the archive. The
[rights evidence report](../generated/v1/reports/acquisition-rights-review.json)
adds pinned upstream revision links and Git blob SHA-1 evidence for nine
highest-priority catalogs. Neither report copies cheat bodies.

## Source-level outcomes

| External catalog | Rights-review finding | Required next evidence |
| --- | --- | --- |
| DAOWAce PS2 | No root LICENSE; README and fork consolidation describe several independent sources | Original ordered-cheat-group author permissions, not merely conversion provenance |
| crashGG PS1 derivative delta | 268 distinct-byte variants relative to DuckStation; pinned LICENSE reserves cheats/patches to original authors and limits MIT to processing scripts | Resolve authorship and individual reuse grants before copying original text |
| DuckStation PS1 | Pinned LICENSE reserves cheat/patch copyrights to original authors; MIT applies to scripts | Separate original code-author grants |
| ibnux Switch | No root LICENSE; README says cheat files are assembled from several upstream databases | Trace original SXOS/EdiZon authors and licenses |
| sthetix Switch | No root LICENSE; README credits GBAtemp, Cheatslips, Hamlet and graphics patch authors | Per-original provenance and permissions |
| ADEMOLA Switch | Root MIT LICENSE exists; README credits many separately authored mods | Determine exactly which original files the MIT grant covers |
| r0ah VitaCheat | No root LICENSE; README describes submissions and CONTRIBUTORS names code authors | Per-PSV author/submission terms |
| PS2-Widescreen staging | 3,632 PCSX2 widescreen PNACH paths, no root LICENSE, README describes a patch staging package | Trace original widescreen code authors and differentiate staging patches from released PCSX2 patches |
| JourneyOver 3DS | No root LICENSE; README encourages CTRPF contributions and Sharkive migration | Original-author terms and code-level Sharkive overlap |

Absence of a root license does **not** prove copyright, disprove all possible
permissions, or establish who owns the material. A root MIT license may apply
only to the aggregation author's work, not third-party code. Original-author
grants can potentially clear a subset without clearing the whole collection.

The new DS multi-format conversion from szTheory (nine linked file occurrences, eight distinct Git blobs) and the Saramagrean PSP compilation (one linked file) also remain unimported. Both depend on code creators beyond the mirror operator: the DS repository's AGPL root notice does not automatically license separately authored historical Action Replay codes, and the PSP collection cites numerous external sources without a blanket license. Their original archive bytes are not copied.

All nine documented findings are **rights unresolved, link-only**. The other external
catalogs remain link-only and are not silently considered cleared.
Git hashes establish byte identity only, not distinct games, gameplay effects,
authorship, or redistribution rights.

## Reproduction and validation

Run `python3 tools/audit_acquisition_rights.py --write` to reproduce the
metadata-only report from `research/rights-evidence-v1.json`, the pinned
source manifests, and the current acquisition priorities. `--check` compares
against the committed artifact. Every documentary witness has an immutable
upstream commit URL and an exact source Git blob SHA-1, allowing human follow-up.

The full index build validates the evidence/manifest consistency offline.
The dedicated **Verify pinned acquisition rights documents** workflow also
re-fetches only the nine pinned upstream Git *root tree listings* and confirms
that every cited README/LICENSE/contributor Git blob identity, and each observed
root license filename list, matches the immutable revision. It never downloads
cheat files or code bodies. This checks what the documents were, **not**
whether every code author granted redistribution permission. The report is
included in the SHA-256 distribution.


## Additional link-only original archive witnesses

Two Libretro-hosted MAME arcade cheat archives (`mame2010-libretro/metadata/cheat.zip`, 24,981,772 bytes; `mame2003-plus-libretro/metadata/cheat.dat`, 7,810,253 bytes) and 48 novel nonempty PNACH files from the `shadowninja826` PS2 mirror also remain link-only. For MAME, the original emulator-license files are not a contributor-by-contributor clearance of cheat bodies; the MAME 2010 ZIP exactly matches one earlier mirror. The PS2 collection offers no root blanket grant and most of its files exactly duplicate a pinned catalog. The rights-evidence report continues to contain nine explicitly documented pinned source dossiers, **not** blanket approval or a complete legal review of all 30 external catalog IDs.
