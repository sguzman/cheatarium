#!/usr/bin/env python3
"""Bulk-preserve the complete GPL-licensed CheatDevicePS2 source database.

The original 6.4 MB database stays byte-identical and game records are indexed
automatically in Rust; this script does not curate games or modify the source.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ID = "root670-cheatdevice-ps2"
REPOSITORY = "https://github.com/root670/CheatDevicePS2"
UPSTREAM = REPOSITORY + ".git"
REVISION = "7d9c525a3a8377ff0b8fe782ea692ed0f245b92c"
FILE = "CheatDatabase.txt"
EXPECTED_BLOB = "e2dcbafa32b8e955ceb23c79842a6098d393fd97"
EXPECTED_BYTES = 6375204
SELECTED = ("CheatDatabase.txt", "LICENSE", "README.md")
MANIFEST = ROOT / "sources" / (ID + ".json")

def blob_sha(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()

def acquire(root):
    actual = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if actual != REVISION:
        raise RuntimeError(f"Wrong pinned CheatDevicePS2 revision: {actual}")
    raw = (root / FILE).read_bytes()
    if len(raw) != EXPECTED_BYTES or blob_sha(raw) != EXPECTED_BLOB:
        raise RuntimeError("Original PS2 cheat database changed from audited source")
    old = {}
    if MANIFEST.exists():
        prior = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if prior.get("snapshot_commit") != REVISION:
            raise RuntimeError("Cannot mix pinned PS2 source snapshots")
        old = {x["upstream_path"]: x for x in prior["files"]}
    entries = []
    created = 0
    for relative in SELECTED:
        source = root / relative
        if not source.is_file() or source.is_symlink():
            raise RuntimeError(f"Missing original source/notice: {relative}")
        payload = source.read_bytes()
        item = {
            "upstream_path": relative,
            "archive_path": f"archive/{ID}/{relative}",
            "git_blob_sha": blob_sha(payload),
        }
        if relative in old and old[relative] != item:
            raise RuntimeError(f"Original provenance conflict: {relative}")
        destination = ROOT / item["archive_path"]
        if destination.exists():
            if destination.read_bytes() != payload:
                raise RuntimeError(f"Existing archived original differs: {relative}")
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
            created += 1
        entries.append(item)
    if set(old) != set() and set(old) != set(SELECTED):
        raise RuntimeError("Original source inventory was changed")
    manifest = {
        "schema_version": 1,
        "id": ID,
        "repository": REPOSITORY,
        "snapshot_commit": REVISION,
        "license": "GPL-3.0-only (upstream project notice; historic cheat-code author rights separately unverified)",
        "archive_prefix": f"archive/{ID}/",
        "scope": "Complete original PlayStation 2 CheatDevice game-cheat database and original notices",
        "rights_note": "Bundled with GPL-3.0 CheatDevicePS2; the original game codes may be derived from historic independent authors. License and original bytes are preserved; no inference of code authorship, release compatibility or successful execution.",
        "original_game_sections": 1483,
        "source_cheat_files": 1,
        "files": entries,
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({"revision": REVISION, "original_database_bytes": len(raw),
                      "new_archived_files_including_notices": created,
                      "original_game_sections": 1483, "source_cheat_files": 1}, indent=2))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path)
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-ps2-db-") as tmp:
        upstream = Path(tmp) / "CheatDevicePS2"
        subprocess.run(["git", "clone", "--quiet", "--depth", "1",
                        "--filter=blob:none", UPSTREAM, str(upstream)], check=True)
        subprocess.run(["git", "-C", str(upstream), "fetch", "--quiet", "--depth", "1",
                        "origin", REVISION], check=True)
        subprocess.run(["git", "-C", str(upstream), "checkout", "--quiet", "--detach",
                        REVISION], check=True)
        acquire(upstream)

if __name__ == "__main__":
    main()
