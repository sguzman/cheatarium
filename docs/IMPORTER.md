# Libretro importer

`tools/import_libretro.py` copies original `.cht` files from a **pinned** Libretro git commit into `archive/libretro/cht/` and records every imported file's original relative path and Git blob SHA in `sources/libretro-database.json`.

It uses a temporary sparse checkout: it does **not** clone ROMs or take the entire Libretro working tree. There are no Python package dependencies; `git` and network access are required.

```sh
python3 tools/import_libretro.py --systems nes snes
python3 tools/validate.py
```

For a bounded trial use `--limit 25` (per system). The default is the full system collection. The import is repeatable and refuses to overwrite a modified archived file. Additional supported identifiers are listed by `--help`.

The repository's one-time GitHub Actions bootstrap runs the NES/SNES import on changes to its workflow or importer, then commits the result only if validation succeeds. This is *not* a continuous scraper. Wider platforms require intentional scope and source reviews.
