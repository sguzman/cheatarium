#!/usr/bin/env python3
"""Archive pinned Gecko/AR game-cheat originals for GameCube, Wii and Wii U.

Only original cheat-bearing GameINI files / author-generated code documents
and their upstream usage/credit notices are collected. No emulator binaries,
executables, or game images.
"""
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
COLLECTIONS = (
    {
        "id": "admentus-enhancement-codes",
        "url": "https://github.com/Admentus64/Enhancement-Codes.git",
        "repository": "https://github.com/Admentus64/Enhancement-Codes",
        "commit": "f8e9ad12d90460e316b552fdf73052eb308e9ab9",
        "license": "GPL-3.0-only (repository license; respect named source-code authors and upstream notices)",
        "expected_cheat_files": 21,
        "notices": ("LICENSE", "README.md", "Credits.txt"),
        "rights_note": (
            "The upstream Credits.txt attributes AR/Gecko coding to numerous authors. "
            "These credits and original per-game Dolphin .ini contents are preserved."
        ),
    },
    {
        "id": "mkwcat-gecko-codes",
        "url": "https://github.com/mkwcat/gecko-codes.git",
        "repository": "https://github.com/mkwcat/gecko-codes",
        "commit": "5b31c6c18bc0982fcbc1cdd9346631abd0d7d29b",
        "license": "MIT (upstream explicitly licenses the original Gecko codes and requests author credit)",
        "expected_cheat_files": 28,
        "notices": ("LICENSE", "README.md", "nsmbu/README.md"),
        "rights_note": (
            "The upstream author explicitly licenses their own Wii/Wii U Gecko "
            "codes under MIT and asks for credit. Original version-specific "
            "markdown and its requirements remain unaltered."
        ),
    },
)


def blob_sha(contents: bytes):
    return hashlib.sha1(f"blob {len(contents)}\0".encode() + contents).hexdigest()


def get_original_paths(folder: Path):
    raw = subprocess.check_output(
        ["git", "-C", str(folder), "ls-tree", "-r", "-z", "--name-only", "HEAD"]
    )
    return [
        name.decode("utf-8", errors="surrogateescape")
        for name in raw.split(b"\0") if name
    ]


def wanted_paths(spec, all_paths):
    notices = set(spec["notices"])
    out = set()
    cheat_files = 0
    for relative in all_paths:
        parts = PurePosixPath(relative).parts
        if not parts or ".." in parts:
            raise RuntimeError(f"Invalid original path: {relative}")
        if relative in notices:
            out.add(relative)
            continue
        if spec["id"] == "admentus-enhancement-codes":
            if relative.endswith(".ini"):
                if len(parts) != 2:
                    raise RuntimeError(f"Unexpected GameINI path: {relative}")
                out.add(relative)
                cheat_files += 1
            elif len(parts) == 2 and parts[-1] == "ReadMe.txt":
                out.add(relative)  # game-specific original instructions
        else:
            if len(parts) == 2 and parts[0] in ("mkw", "nsmbu", "nsmbw"):
                if relative.endswith(".md") and relative != "nsmbu/README.md":
                    out.add(relative)
                    cheat_files += 1
    if cheat_files != spec["expected_cheat_files"]:
        raise RuntimeError(
            f"{spec['id']}: expected {spec['expected_cheat_files']} cheat files, got {cheat_files}"
        )
    if not notices.issubset(out):
        raise RuntimeError(f"{spec['id']}: missing license/attribution notice")
    return sorted(out)


def archive_one(spec, worktree):
    git_revision = subprocess.check_output(
        ["git", "-C", str(worktree), "rev-parse", "HEAD"], text=True
    ).strip()
    if git_revision != spec["commit"]:
        raise RuntimeError(f"{spec['id']}: checked out unexpected revision {git_revision}")
    paths = wanted_paths(spec, get_original_paths(worktree))
    root = ROOT / "archive" / spec["id"]
    dest_manifest = ROOT / "sources" / f"{spec['id']}.json"
    old = {}
    if dest_manifest.exists():
        prior = json.loads(dest_manifest.read_text(encoding="utf-8"))
        if prior["snapshot_commit"] != spec["commit"]:
            raise RuntimeError("Existing manifest uses another source revision")
        old = {x["upstream_path"]: x for x in prior["files"]}
    files = []
    newly_archived = 0
    for relative in paths:
        source = worktree / relative
        if not source.is_file() or source.is_symlink():
            raise RuntimeError(f"Unexpected non-file upstream path: {relative}")
        raw = source.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty original file: {relative}")
        record = {
            "upstream_path": relative,
            "archive_path": f"archive/{spec['id']}/{relative}",
            "git_blob_sha": blob_sha(raw),
        }
        if relative in old and old[relative] != record:
            raise RuntimeError(f"Original source changed within pinned revision: {relative}")
        target = root / relative
        if target.exists():
            if target.read_bytes() != raw:
                raise RuntimeError(f"Previously archived source differs: {relative}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            newly_archived += 1
        files.append(record)
    if set(old).difference({f["upstream_path"] for f in files}):
        raise RuntimeError("An existing archival source was dropped")
    doc = {
        "schema_version": 1,
        "id": spec["id"],
        "repository": spec["repository"],
        "snapshot_commit": spec["commit"],
        "license": spec["license"],
        "archive_prefix": f"archive/{spec['id']}/",
        "source_cheat_files": spec["expected_cheat_files"],
        "platform_scope": ["gamecube", "wii", "wii-u"]
            if spec["id"] == "admentus-enhancement-codes" else ["wii", "wii-u"],
        "rights_note": spec["rights_note"],
        "files": files,
    }
    dest_manifest.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return {
        "source": spec["id"],
        "cheat_files": spec["expected_cheat_files"],
        "preserved_originals_including_notices": len(files),
        "new_files": newly_archived,
    }


def main():
    results = []
    with tempfile.TemporaryDirectory(prefix="cheatarium-gecko-sources-") as tmp:
        for spec in COLLECTIONS:
            checkout = Path(tmp) / spec["id"]
            subprocess.run([
                "git", "clone", "--quiet", "--depth", "1", "--filter=blob:none",
                spec["url"], str(checkout)
            ], check=True)
            subprocess.run([
                "git", "-C", str(checkout), "fetch", "--quiet", "--depth", "1",
                "origin", spec["commit"]
            ], check=True)
            subprocess.run([
                "git", "-C", str(checkout), "checkout", "--quiet", "--detach", spec["commit"]
            ], check=True)
            results.append(archive_one(spec, checkout))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
