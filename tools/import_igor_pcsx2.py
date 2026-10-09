#!/usr/bin/env python3
"""Preserve the pinned, separately authored Igor PCSX2 source collection.

The project declares MIT and all 17 .pnach files are copied byte-for-byte,
keeping regional filenames and raw PCSX2 patch syntax without equivalence
claims. Root author license and README are preserved as original source files.
"""
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
REPO = "https://github.com/igorciz777/Misc-PCSX2-Cheats"
COMMIT = "26c4395dd3c10d469168c12c5b9f08e56f4de061"
SID = "igor-misc-pcsx2-cheats"
TARGET = ROOT / "archive" / SID
MANIFEST = ROOT / "sources" / f"{SID}.json"
EXPECTED_CHEATS = 17


def git_sha(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def acquire(root):
    actual = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True,
    ).strip()
    if actual != COMMIT:
        raise RuntimeError(f"Invalid source revision: {actual}")
    files = sorted(subprocess.check_output(
        ["git", "-C", str(root), "ls-tree", "-r", "--name-only", "HEAD"],
        text=True,
    ).splitlines())
    if len(files) != 19 or files.count("LICENSE") != 1 or files.count("README.md") != 1:
        raise RuntimeError("Incomplete pinned original source inventory")
    if sum(name.endswith(".pnach") for name in files) != EXPECTED_CHEATS:
        raise RuntimeError("Incomplete native PS2 cheat collection")
    old = {}
    if MANIFEST.exists():
        prior = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if prior.get("id") != SID or prior.get("snapshot_commit") != COMMIT:
            raise RuntimeError("Cannot silently replace pinned source snapshot")
        old = {row["upstream_path"]: row for row in prior["files"]}
    entries = []
    new_files = 0
    for path in files:
        rel = PurePosixPath(path)
        if rel.is_absolute() or not rel.parts or any(x in ("", ".", "..") for x in rel.parts):
            raise RuntimeError(f"Unexpected upstream path: {path!r}")
        src = root / path
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Unexpected upstream file type: {path}")
        raw = src.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty original file: {path}")
        record = {
            "upstream_path": path,
            "archive_path": f"archive/{SID}/{path}",
            "git_blob_sha": git_sha(raw),
        }
        if path in old and old[path] != record:
            raise RuntimeError(f"Original provenance changed: {path}")
        dest = TARGET / path
        if dest.exists():
            if not dest.is_file() or dest.read_bytes() != raw:
                raise RuntimeError(f"Byte mismatch in preserved archive: {path}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(raw)
            new_files += 1
        entries.append(record)
    if set(old).difference(files):
        raise RuntimeError("Previously imported original files disappeared")
    MANIFEST.write_text(json.dumps({
        "schema_version": 1,
        "id": SID,
        "repository": REPO,
        "snapshot_commit": COMMIT,
        "license": "MIT (original author copyright notice preserved)",
        "archive_prefix": f"archive/{SID}/",
        "scope_note": "17 region- and CRC-specific PCSX2 .pnach files; exact version compatibility is not independently verified by Cheatarium.",
        "rights_note": "Preserve original Igor README and MIT LICENSE; do not conflate patches for different region/release IDs.",
        "native_cheat_counts": {"ps2": EXPECTED_CHEATS},
        "files": entries,
    }, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Imported {EXPECTED_CHEATS} PS2 cheat files; {len(entries)} original upstream files; {new_files} new copies")


def main():
    with tempfile.TemporaryDirectory(prefix="cheatarium-igor-") as temp:
        root = Path(temp) / "upstream"
        subprocess.run(["git", "clone", "--quiet", "--filter=blob:none",
                        "--depth", "1", "--no-checkout", f"{REPO}.git", str(root)], check=True)
        subprocess.run(["git", "-C", str(root), "fetch", "--quiet", "--depth", "1",
                        "origin", COMMIT], check=True)
        subprocess.run(["git", "-C", str(root), "checkout", "--quiet", "--detach", COMMIT], check=True)
        acquire(root)


if __name__ == "__main__":
    main()
