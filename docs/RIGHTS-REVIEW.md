# Bulk cheat acquisition: pinned original-author rights review

The project currently archives 67,834 original files and separately indexes
38,573 **link-only** external occurrences. Not all are cheats, unique titles,
or independently verified modifications.

The [priority scorecard](../generated/v1/reports/acquisition-priorities.json)
ranks sources by exact Git blob bytes absent from the archive. The
[rights evidence report](../generated/v1/reports/acquisition-rights-review.json)
adds pinned upstream revision links and Git blob SHA-1 evidence for seven
highest-priority catalogs. Neither report copies cheat bodies.

## Source-level outcomes

| External catalog | Rights-review finding | Required next evidence |
| --- | --- | --- |
| DAOWAce PS2 | No root LICENSE; README and fork consolidation describe several independent sources | Original ordered-cheat-group author permissions, not merely conversion provenance |
| DuckStation PS1 | Pinned LICENSE reserves cheat/patch copyrights to original authors; MIT applies to scripts | Separate original code-author grants |
| ibnux Switch | No root LICENSE; README says cheat files are assembled from several upstream databases | Trace original SXOS/EdiZon authors and licenses |
| sthetix Switch | No root LICENSE; README credits GBAtemp, Cheatslips, Hamlet and graphics patch authors | Per-original provenance and permissions |
| ADEMOLA Switch | Root MIT LICENSE exists; README credits many separately authored mods | Determine exactly which original files the MIT grant covers |
| r0ah VitaCheat | No root LICENSE; README describes submissions and CONTRIBUTORS names code authors | Per-PSV author/submission terms |
| JourneyOver 3DS | No root LICENSE; README encourages CTRPF contributions and Sharkive migration | Original-author terms and code-level Sharkive overlap |

Absence of a root license does **not** prove copyright, disprove all possible
permissions, or establish who owns the material. A root MIT license may apply
only to the aggregation author's work, not third-party code. Original-author
grants can potentially clear a subset without clearing the whole collection.

All seven findings are **rights unresolved, link-only**. The other external
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
re-fetches only the seven pinned upstream Git *root tree listings* and confirms
that every cited README/LICENSE/contributor Git blob identity, and each observed
root license filename list, matches the immutable revision. It never downloads
cheat files or code bodies. This checks what the documents were, **not**
whether every code author granted redistribution permission. The report is
included in the SHA-256 distribution.
