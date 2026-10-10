#!/usr/bin/env python3
"""Preserve AeroWidescreen's pinned MIT-licensed PS2 PNACH cheats verbatim.

This is an archival importer: it does not execute patches or assert that
their reported game, revision, CRC, or gameplay effects are independently tested.
"""
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
REPO = "https://github.com/AeroWidescreen/PCSX2-Cheats"
COMMIT = "ab043ba5adb9e65837fd451d207e6c7c2731c3b1"
SID = "aerowidescreen-pcsx2-cheats"
ARCHIVE = ROOT / "archive" / SID
MANIFEST = ROOT / "sources" / f"{SID}.json"
EXPECTED_CHEATS = 62
EXPECTED_FILES = 65
NOTICES = {".gitattributes", "LICENSE", "README.md"}


def blob_sha(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def acquire(tree):
    rev = subprocess.check_output(
        ["git", "-C", str(tree), "rev-parse", "HEAD"], text=True
    ).strip()
    if rev != COMMIT:
        raise RuntimeError(f"Unexpected upstream revision: {rev}")
    paths = sorted(subprocess.check_output(
        ["git", "-C", str(tree), "ls-tree", "-r", "--name-only", "HEAD"], text=True
    ).splitlines())
    cheats = [p for p in paths if p.lower().endswith(".pnach")]
    if (len(paths) != EXPECTED_FILES or len(set(paths)) != len(paths)
            or len(cheats) != EXPECTED_CHEATS
            or set(paths).difference(cheats) != NOTICES):
        raise RuntimeError("Incomplete or unexpectedly changed pinned PS2 collection")
    old = {}
    if MANIFEST.is_file():
        previous = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if previous.get("id") != SID or previous.get("snapshot_commit") != COMMIT:
            raise RuntimeError("Refusing to merge another author or source revision")
        old = {e["upstream_path"]: e for e in previous["files"]}
    entries = []
    new_files = 0
    for name in paths:
        p = PurePosixPath(name)
        if p.is_absolute() or not p.parts or any(x in ("", ".", "..") for x in p.parts):
            raise RuntimeError(f"Unsafe upstream path: {name!r}")
        src = tree / name
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Not a regular upstream file: {name}")
        raw = src.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty original file: {name}")
        record = {
            "upstream_path": name,
            "archive_path": f"archive/{SID}/{name}",
            "git_blob_sha": blob_sha(raw),
        }
        if name in old and old[name] != record:
            raise RuntimeError(f"Previously imported source changed: {name}")
        dest = ARCHIVE / name
        if dest.exists():
            if not dest.is_file() or dest.read_bytes() != raw:
                raise RuntimeError(f"Archived source diverges from pinned original: {name}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(raw)
            new_files += 1
        entries.append(record)
    if set(old).difference(paths):
        raise RuntimeError("Previously imported originals would be removed")
    manifest = {
        "schema_version": 1,
        "id": SID,
        "repository": REPO,
        "snapshot_commit": COMMIT,
        "license": "MIT (root LICENSE; original file-level author text preserved)",
        "archive_prefix": f"archive/{SID}/",
        "scope_note": "62 original PS2 PCSX2 PNACH files: widescreen, controls, access, event and other game patches. Region, CRC, patch activation and compatibility unverified by Cheatarium.",
        "rights_note": "Preserve upstream author's original MIT LICENSE, README and all file-level author, title, version and patch comments. Do not claim these patches are unique, already executed, or interoperable across regions.",
        "native_cheat_counts": {"ps2": EXPECTED_CHEATS},
        "files": entries,
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({
        "source_revision": COMMIT, "imported_new_original_files": new_files,
        "ps2_pnach_files": len(cheats), "all_source_files_including_notices": len(paths)
    }, indent=2))


def main():
    with tempfile.TemporaryDirectory(prefix="cheatarium-aero-") as tmp:
        root = Path(tmp) / "upstream"
        subprocess.run(
            ["git", "clone", "--quiet", "--depth", "1", "--filter=blob:none",
             "--no-checkout", REPO + ".git", str(root)], check=True
        )
        subprocess.run(
            ["git", "-C", str(root), "fetch", "--quiet", "--depth", "1", "origin", COMMIT],
            check=True
        )
        subprocess.run(
            ["git", "-C", str(root), "checkout", "--quiet", "--detach", COMMIT],
            check=True
        )
        acquire(root)


if __name__ == "__main__":
    main()
