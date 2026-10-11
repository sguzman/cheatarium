#!/usr/bin/env python3
"""Bulk-preserve both complete pinned rinCheat PS Vita source databases.

Source-native memory cheat files and save-editor Lua scripts are independent
formats. Preserve exact bytes, names and contributor credit; never execute code.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = "https://github.com/Rinnegatamante/rinCheat.git"
REVISION = "d17fd2a7a542346e503afd9cd1df8c5e8bd33626"
ID = "rincheat-vita"
DEST = ROOT / "archive" / ID
MANIFEST = ROOT / "sources" / f"{ID}.json"
EXPECTED = {"cheats_db": 7, "SE_cheats_db": 51}
NOTICES = {"LICENSE.txt", "README.md"}

def git_blob(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()

def acquire(root):
    actual = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if actual != REVISION:
        raise RuntimeError(f"Pinned source mismatch: {actual}")
    names = subprocess.check_output([
        "git", "-C", str(root), "ls-tree", "-r", "-z", "--name-only", "HEAD"
    ]).split(b"\0")
    selected = set(NOTICES)
    counts = {k: 0 for k in EXPECTED}
    ids = set()
    for encoded in names:
        if not encoded:
            continue
        path = encoded.decode("utf-8")
        parts = PurePosixPath(path).parts
        if not parts or ".." in parts or PurePosixPath(path).is_absolute():
            raise RuntimeError(f"Unsafe upstream path: {path}")
        if len(parts) != 2 or parts[0] not in EXPECTED:
            continue
        stem, suffix = PurePosixPath(parts[1]).stem, PurePosixPath(parts[1]).suffix
        if not (len(stem) == 9 and stem[:3] == "PCS" and stem[3].isalpha()
                and stem[4:].isdigit()):
            raise RuntimeError(f"Unexpected native Vita game key: {path}")
        wanted = ".txt" if parts[0] == "cheats_db" else ".lua"
        if suffix.lower() != wanted:
            raise RuntimeError(f"Unexpected native Vita source type: {path}")
        selected.add(path)
        counts[parts[0]] += 1
        ids.add(stem)
    if counts != EXPECTED or len(ids) != 57:
        raise RuntimeError(f"Pinned Vita inventory changed: {counts}; ids={len(ids)}")
    old = {}
    if MANIFEST.exists():
        prior = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if prior.get("snapshot_commit") != REVISION:
            raise RuntimeError("Cannot mix different upstream Vita revisions")
        old = {item["upstream_path"]: item for item in prior["files"]}
    inventory = []
    created = 0
    for relative in sorted(selected):
        original = root / relative
        if not original.is_file() or original.is_symlink():
            raise RuntimeError(f"Missing or invalid original: {relative}")
        raw = original.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty original: {relative}")
        record = {
            "upstream_path": relative,
            "archive_path": f"archive/{ID}/{relative}",
            "git_blob_sha": git_blob(raw),
        }
        if relative in old and old[relative] != record:
            raise RuntimeError(f"Pinned origin changed: {relative}")
        destination = DEST / relative
        if destination.exists():
            if destination.read_bytes() != raw:
                raise RuntimeError(f"Local original has been altered: {relative}")
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(original, destination)
            created += 1
        inventory.append(record)
    if set(old) - {x["upstream_path"] for x in inventory}:
        raise RuntimeError("An archived Vita original was dropped")
    data = {
        "schema_version": 1,
        "id": ID,
        "repository": "https://github.com/Rinnegatamante/rinCheat",
        "snapshot_commit": REVISION,
        "license": "GPL-3.0-only (upstream LICENSE.txt; external cheat contributors credited separately)",
        "archive_prefix": f"archive/{ID}/",
        "scope": "Full original PS Vita rinCheat runtime-memory and save-edit Lua cheat databases",
        "rights_note": "Project README explicitly credits Slade and other cheat authors. Preserve original source files and GPL text; claims about individual creator rights or tested execution remain unverified.",
        "native_format_counts": counts,
        "source_title_ids": len(ids),
        "source_cheat_files": sum(counts.values()),
        "files": inventory,
    }
    MANIFEST.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({"revision": REVISION, "new_files_including_notices": created,
                      "native_cheat_files": sum(counts.values()), "source_title_ids": len(ids),
                      "by_source_directory": counts}, indent=2))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path)
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-rincheat-") as temp:
        upstream = Path(temp) / "rinCheat"
        subprocess.run(["git", "clone", "--quiet", "--depth", "1",
                        "--filter=blob:none", UPSTREAM, str(upstream)], check=True)
        subprocess.run(["git", "-C", str(upstream), "fetch", "--quiet", "--depth", "1",
                        "origin", REVISION], check=True)
        subprocess.run(["git", "-C", str(upstream), "checkout", "--quiet", "--detach",
                        REVISION], check=True)
        acquire(upstream)

if __name__ == "__main__":
    main()
