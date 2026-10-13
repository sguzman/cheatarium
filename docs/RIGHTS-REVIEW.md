# Bulk cheat acquisition: pinned original-author rights review

The project currently archives 67,834 original files and separately indexes
42,718 **link-only** external occurrences. Not all are cheats, unique titles,
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


Smoker1's 98 FinalBurn Alpha INI file references (45 distinct source blobs) and pacnpal's one Mupen64Plus N64 native cheat database are now searchable metadata-only, not preserved body files. The former README identifies them as externally sourced official FBA cheat listings; the latter describes modifications atop preexisting Mupen/GameShark FAQ code collections. Neither source establishes blanket reuse permissions for all underlying authors. Keep FBA, FBNeo, MAME, and N64 format/compatibility claims distinct. The nine detailed legal evidence dossiers do not automatically cover these two new sources.


The seven new folder-scoped historical RetroArch mirror catalogs preserve **86 link-only file occurrences / 77 Git byte hashes** selected from 11,390 original upstream files using an earlier exact-blob comparison with Libretro/Emupedia/BitHeX. Unlike the original upstream homebrew secrets collection, these older CHT forks do not establish individual code-author permission to republish cheat bodies. Their platform labels are based on source folder names; they are not verification of game identity, effects or compatible ROM versions.

### Measured internal code redundancy (2026-10-13)

A checksum-verified aggregate-only [compiled-source audit](../research/compiled-source-identity-audit-v1.json)
found 75,036 exact source-serial + normalized command-block pairs shared
between two pinned PSP CWCheat collections; the source-only differences are
2,281 (Saramagrean) and 1,586 (ApelFactory). This **does not establish
redistribution rights for either collection** or prove version compatibility.
The Nintendo SQLite dataset and its four JSON exports have exactly matching
5,327/117,861 source-declared game/cheat row totals, so republication in multiple
file formats must not increase unique-content totals. Distinct Sega CD ZIP
parents contain 385 members but only 344 byte-distinct member bodies.
Attribution and reuse clearance must be reviewed at the original creator
level, not inferred from repackaging or matching content.

### MAME 2010 source alias inflation (2026-10-13)

The original cheat.zip contains **8,837 original XML file occurrences but
4,108 distinct XML file bodies** and **106,805 cheat elements but only
33,400 different serialized XML cheat subtrees**, measured by a reproducible
[ephemeral pinned archive audit](../research/mame-xml-structural-audit-v1.json).
This does not establish 33,400 unique effects, permissions, or game/build
compatibility. The source's upstream emulator license does not independently
clear historic cheat authors. Keep original XML content and machine-readable
codes out of the archive pending actual author-level reuse evidence.

### MAME historical DAT/XML overlap labels (2026-10-13)

The link-only MAME 2003-Plus DAT has **546 strictly recognized native
driver labels**, **493** of which match pinned MAME 2010 XML filename
stems exactly; **53** occur only in that DAT. This is label overlap
only; the [checksum-verified report](../research/mame-dat-xml-driver-labels-v1.json)
publishes no cheat records or per-game identifiers. No source-native
MAME DAT / XML label match can be used as copyright, original-authorship,
game-identity or executable compatibility evidence.

### Nintendo GameCube / Wii U regional bulk sources (2026-10-13)

A pinned [author-rights evidence update](../research/rights-evidence-v1.json)
covers three newly indexed source families. The LeumasPlays WiiRD GameCube
regional ZIPs are compiled from third-party gc-forever posts; no repository
LICENSE was observed and the compiler admits limited game testing. The
KatsuMaki1 Wii U/Cemu XML group cites GBAtemp/Discord and individual code
authors; no root license was observed. Vincent Coding's uCodes website
publishes an MIT LICENSE for the software but the original Wii U Cafe/Gecko
cheat records carry independent author-credit fields. Do not transfer the
software license across separately credited contributions without evidence.
All three are **rights-unresolved links**; imported original cheat bodies = 0.

### GameCube/Wii U actual contents measured (2026-10-13)

The [read-only aggregate audit](../research/nintendo-bulk-member-coverage-v1.json)
verified 375 GameCube WiiRD source text members inside the three
regional ZIP containers, 97 Wii U XML entries and 50 Wii U JSON record
occurrences. At least 90 of the XML entries bear source-credit author
labels and 47 of the JSON records include author labels. This
confirms contributor-level authorship review is still material.
None of these counts establishes permission to redistribute original
cheat-code text. No original source codes were committed.

### Wii native original-code provenance (2026-10-13)

The rights audit now documents four additional original Wii source READMEs (Git SHA-1 pinned) for City Folk/Deluxe, Metroid Prime/Trilogy, Metroid: Other M, and Taiko. These four families comprise **29** original Gecko code files, not 29 verified game effects. Upstream author attributions and public GitHub storage alone cannot establish redistribution grants for each original code author. Keep their original executable revision hints and platform distinctions; no third-party code bodies are republished as a result of this discovery.
