# Private Switch source collection

Cheatarium's permanent source collection is independent of emulator installation.
Pinned, redistributable originals belong in `archive/`; source code whose
redistribution permissions have not been established is retained locally,
outside the public Git working tree. Source references and reviewed rights
metadata may still live in the repository.

## Local originals and index

The Switch acquisition commands store complete native `.txt` code bodies by
Title ID and exact Build ID under
`~/.local/share/cheatarium/private-switch-curation/<TITLE_ID>/<BUILD_ID>.txt`.
External sources additionally have adjacent `.source.json` receipts with
the attributed source URL, exact code checksum and section count.

Acquisition refreshes `collection-index.json` in the same private directory.
The index contains named sections, exact game and Build IDs, source provenance,
byte counts, SHA-256 hashes and explicit *not gameplay verified* status.
The original code files remain authoritative; the index is regenerable.

An existing file with different bytes is **never silently overwritten**.
A failed or missing Eden installation does not remove locally acquired codes.

## First backup and persistent configuration

Choose an independent filesystem and run:

```sh
python3 tools/private_switch_collection.py --backup-to /absolute/path/to/independent/backup --remember-backup
```

This copies every acquired native source file and source receipt, then writes a
content-addressed, immutable JSON index snapshot. Each future acquisition
automatically refreshes the local index and repeats the non-overwriting
backup using the saved destination. A matching backup is reused; a changed or
corrupt destination produces an error rather than destroying either copy.
For backup paths under `/drive`, the tool refuses to proceed when that mount is
not active. The backup target must not be inside the private source collection.

For offline verification:

```sh
python3 tools/private_switch_collection.py --verify-backup
```

For a quick catalog or case-insensitive name search:

```sh
python3 tools/private_switch_collection.py
python3 tools/private_switch_collection.py --search "invincible"
```

Neither searching nor backup verification touches Eden or the network.
The source files and private catalog are never published as Git commits or
GitHub Actions artifacts. This tool only manages the **private Switch cache**;
it does not claim to have imported those files into the public aggregated
`generated/v1` catalog, nor does it prove runtime compatibility.

For recovery on a replacement machine, the backup directory contains the same
Title ID / Build ID source layout, source receipts, and immutable JSON index
snapshots. Restore the source directory and rerun the catalog command;
Eden add-ons can then be regenerated from those originals.
