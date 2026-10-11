# EmuCoreX Cheat Catalog

Community-maintained PNACH catalog consumed by the EmuCoreX cheat manager.

The repository stores catalog metadata and validation tools. PNACH files are
mirrored when redistribution permits it; otherwise the catalog uses immutable
upstream commit URLs. Every entry preserves authorship and its original source
URL while branch changes cannot alter the reviewed file.

## Files

- `cheats.json` - production catalog used by EmuCoreX.
- `sources.json` - approved upstream collections and their attribution.
- `schemas/cheat-catalog.schema.json` - public format contract.
- `files` - PNACH mirrors grouped by credited upstream source.
- `scripts/import_pcsx2_collection.py` - reproducible multi-source catalog importer.
- `scripts/build_pcsx2_curated_batch.py` - strict audited batch builder for pinned PCSX2 patches.
- `scripts/validate_catalog.py` - dependency-free catalog validator.
- `scripts/validate_audits.py` - verifies pinned non-mirrored batches and content-hash deduplication.
- `scripts/verify_audit_live.py` - downloads and strictly checks every external file in an audit.
- `audits` - immutable source, size and SHA-256 records for curated batches.
- `frame-generation/support-v1.json` - revocable compatibility manifest for the
  optional EmuCoreX LSFG integration. It contains no proprietary shaders or DLLs;
  users must import their own legally obtained `Lossless.dll` from Lossless Scaling.
- `LICENSES` - license texts for redistributed or generated source-derived packs.

## Updating the upstream catalog

```bash
python scripts/import_pcsx2_collection.py
python scripts/validate_catalog.py
python scripts/validate_audits.py
```

The importer pins remote raw files to the exact upstream commit used to build
the catalog, keeps credited alternative collections for the same serial/CRC,
and validates that every selected file contains supported PNACH blocks. The
GPL-3.0 CheatDevice conversion is regenerated from an immutable source database,
then checked against audited output hashes and pinned serial/CRC evidence. The
official PCSX2 patch batch keeps direct immutable upstream URLs, accepts only
new CRCs with strict PNACH syntax, and maps titles from a pinned GameIndex. The
current catalog exposes 9,594 verified entries from sixty-two credited sources; the
app merges additional packs instead of overwriting cheats already installed
for a game.

See [CONTRIBUTING.md](CONTRIBUTING.md) before adding or mirroring content.
