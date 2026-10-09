#!/usr/bin/env python3
"""Acquire Sharkive's pinned 3DS and Switch cheat files without editing originals.

The project repository declares GPL-3.0; retain its license and contributors'
native credit in all copies. Never infer that submitted cheats have been tested.
Requires network access and git for a single pinned checkout. No ROM access.
"""
import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
URL = "https://github.com/FlagBrew/Sharkive.git"
COMMIT = "aeab5fd3b001ed22c013efd1a02b092575d825ed"
MANIFEST = ROOT / "sources/sharkive.json"
ARCHIVE = ROOT / "archive/sharkive"
PATH_RULES = {
    "3ds": re.compile(r"3ds/[a-fA-F0-9]{16}\.txt\Z"),
    "switch": re.compile(r"switch/[a-fA-F0-9]{16}/[a-fA-F0-9]{16}\.txt\Z"),
}
EXPECTED = {"3ds": 644, "switch": 491}


def git_blob_sha(data):
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()


def import_snapshot(upstream):
    checked_out = subprocess.check_output(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True
    ).strip()
    if checked_out != COMMIT:
        raise RuntimeError(f"Source snapshot mismatch: {checked_out} != {COMMIT}")
    if MANIFEST.exists():
        previous = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if previous.get("snapshot_commit") != COMMIT:
            raise RuntimeError("Refusing to merge unlike upstream source revisions")
        old = {entry["upstream_path"]: entry for entry in previous["files"]}
    else:
        old = {}
    selected = [upstream / "LICENSE"]
    stats = {}
    for platform, rule in PATH_RULES.items():
        matches = []
        for path in (upstream / platform).rglob("*.txt"):
            rel = path.relative_to(upstream).as_posix()
            if not rule.fullmatch(rel) or path.is_symlink() or not path.is_file():
                raise RuntimeError(f"Unexpected file layout/type in {platform}: {rel}")
            matches.append(path)
        matches.sort()
        if len(matches) != EXPECTED[platform]:
            raise RuntimeError(
                f"Incomplete pinned {platform} snapshot: expected {EXPECTED[platform]}, got {len(matches)}"
            )
        stats[platform] = {
            "source_files": len(matches),
            "original_section_headings": 0,
        }
        selected.extend(matches)
    new_files = 0
    inventory = []
    for path in selected:
        if path.is_symlink() or not path.is_file():
            raise RuntimeError(f"Expected unchanged regular upstream file: {path}")
        rel = path.relative_to(upstream).as_posix()
        raw = path.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty upstream source file: {rel}")
        dest = ARCHIVE / rel
        record = {
            "upstream_path": rel,
            "archive_path": f"archive/sharkive/{rel}",
            "git_blob_sha": git_blob_sha(raw),
        }
        if rel in old and old[rel] != record:
            raise RuntimeError(f"Previously imported source changed: {rel}")
        if dest.exists():
            if dest.read_bytes() != raw:
                raise RuntimeError(f"Archived source bytes differ from pin: {rel}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, dest)
            new_files += 1
        if rel.endswith(".txt"):
            platform = rel.split("/", 1)[0]
            stats[platform]["original_section_headings"] += len(
                re.findall(rb"(?m)^\[[^\]\r\n]+\]\s*$", raw)
            )
        inventory.append(record)
    manifest = {
        "schema_version": 1,
        "id": "sharkive",
        "repository": "https://github.com/FlagBrew/Sharkive",
        "snapshot_commit": COMMIT,
        "license": "GPL-3.0-only",
        "archive_prefix": "archive/sharkive/",
        "rights_note": (
            "Sharkive's root LICENSE declares GPLv3 for the project. "
            "Original cheat submitters and upstream source credits remain applicable. "
            "Preserve native credit, do not relicense as MIT, and do not treat "
            "repository-level GPL terms as proof about all possible third-party rights."
        ),
        "platforms": stats,
        "files": sorted(inventory, key=lambda item: item["upstream_path"]),
    }
    if set(old).difference({row["upstream_path"] for row in inventory}):
        raise RuntimeError("Pinned source inventory unexpectedly dropped files")
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "snapshot": COMMIT,
        "new_source_files": new_files,
        "total_archived_source_files": len(inventory),
        "platforms": stats,
        "source_license_archived": (ARCHIVE / "LICENSE").is_file(),
    }, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path, default=None,
                        help="Use a pre-checked-out local source tree (tests/controlled imports)")
    args = parser.parse_args()
    if args.checked_out_upstream is not None:
        import_snapshot(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-sharkive-") as tmp:
        upstream = Path(tmp) / "Sharkive"
        subprocess.run(["git", "clone", "--quiet", "--depth", "1", URL, str(upstream)], check=True)
        subprocess.run(["git", "-C", str(upstream), "fetch", "--quiet", "--depth", "1", "origin", COMMIT], check=True)
        subprocess.run(["git", "-C", str(upstream), "checkout", "--quiet", "--detach", COMMIT], check=True)
        import_snapshot(upstream)


if __name__ == "__main__":
    main()
