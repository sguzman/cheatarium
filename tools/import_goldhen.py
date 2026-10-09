#!/usr/bin/env python3
"""Preserve a pinned, full GoldHEN PS4 cheat collection with native provenance.

The upstream repository declares GPL-3.0. Some cheat files derive from
third-party creators; preserve their individual credits without making
unsupported claims about ownership or tested gameplay behavior.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = "https://github.com/GoldHEN/GoldHEN_Cheat_Repository.git"
COMMIT = "acc22fefac0f0ea35a0f59b3f9526707122098dd"
MANIFEST = ROOT / "sources/goldhen.json"
ARCHIVE = ROOT / "archive/goldhen"
EXPECTED = {"json": 1878, "mc4": 1865, "shn": 1702}
NOTICES = ("LICENSE", ".github/README.md", "json.txt", "mc4.txt", "shn.txt")


def blob_sha(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def source_list(worktree: Path) -> dict:
    out = subprocess.check_output(
        ["git", "-C", str(worktree), "ls-tree", "-r", "--name-only", "HEAD"],
        text=True,
    )
    selected = {}
    for relative in out.splitlines():
        posix = PurePosixPath(relative)
        if posix.is_absolute() or ".." in posix.parts:
            raise RuntimeError(f"Unsafe upstream relative path: {relative}")
        if relative in NOTICES:
            selected[relative] = "upstream-notice"
        elif posix.parts and posix.parts[0] in EXPECTED:
            if len(posix.parts) != 2:
                raise RuntimeError(f"Unexpected nested source layout: {relative}")
            if posix.suffix.lower() not in {".json", ".mc4", ".shn", ".xml"}:
                raise RuntimeError(f"Unexpected cheat file type: {relative}")
            selected[relative] = posix.parts[0]
    if set(NOTICES).difference(selected):
        raise RuntimeError(f"Missing upstream notices: {sorted(set(NOTICES).difference(selected))}")
    for directory, count in EXPECTED.items():
        observed = sum(kind == directory for kind in selected.values())
        if observed != count:
            raise RuntimeError(f"Expected {count} {directory} source files, found {observed}")
    return selected


def acquire(root: Path):
    actual = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if actual != COMMIT:
        raise RuntimeError(f"Unexpected Git revision {actual}, expected {COMMIT}")
    selected = source_list(root)
    old = {}
    if MANIFEST.exists():
        previous = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if previous.get("snapshot_commit") != COMMIT:
            raise RuntimeError("Cannot merge different pinned GoldHEN snapshots")
        old = {item["upstream_path"]: item for item in previous["files"]}
    inventory = []
    new_files = 0
    counts = {name: 0 for name in EXPECTED}
    anomalous_names = []
    for rel, kind in sorted(selected.items()):
        origin = root / rel
        if not origin.is_file() or origin.is_symlink():
            raise RuntimeError(f"Expected plain upstream source file: {rel}")
        raw = origin.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty original source file: {rel}")
        record = {
            "upstream_path": rel,
            "archive_path": f"archive/goldhen/{rel}",
            "git_blob_sha": blob_sha(raw),
        }
        if rel in old and old[rel] != record:
            raise RuntimeError(f"Original source diverges from prior import: {rel}")
        target = ARCHIVE / rel
        if target.exists():
            if target.read_bytes() != raw:
                raise RuntimeError(f"Existing archive not byte-identical to upstream: {rel}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(origin, target)
            new_files += 1
        if kind in counts:
            counts[kind] += 1
            name = PurePosixPath(rel).name
            if not name.startswith("CUSA"):
                anomalous_names.append(rel)
        inventory.append(record)
    if set(old).difference({item["upstream_path"] for item in inventory}):
        raise RuntimeError("Source inventory has lost previously imported originals")
    manifest = {
        "schema_version": 1,
        "id": "goldhen",
        "repository": "https://github.com/GoldHEN/GoldHEN_Cheat_Repository",
        "snapshot_commit": COMMIT,
        "license": "GPL-3.0-only (repository LICENSE; third-party cheat author rights require separate attribution review)",
        "archive_prefix": "archive/goldhen/",
        "scope": "GoldHEN-format PS4-oriented source collection; source identifiers can reference PS2-era titles but do not prove native PS2 cheat compatibility",
        "rights_note": (
            "GoldHEN README explicitly credits PS4Trainer and original cheat creators, "
            "and states GoldHEN supports formats rather than developing cheats. "
            "Repository GPL-3 license is preserved verbatim; individual contributor "
            "rights and upstream credits must remain distinct from Cheatarium MIT code."
        ),
        "native_format_counts": counts,
        "non_cusa_named_originals": len(anomalous_names),
        "files": inventory,
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "revision": COMMIT,
        "new_source_files_including_notices": new_files,
        "cheat_source_files": sum(counts.values()),
        "by_directory": counts,
        "notices": len(NOTICES),
        "source_manifest_records": len(inventory),
        "non_cusa_filename_count": len(anomalous_names),
    }, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path, default=None,
                        help="Import from a locally pinned upstream checkout")
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-goldhen-") as temp:
        upstream = Path(temp) / "GoldHEN_Cheat_Repository"
        subprocess.run([
            "git", "clone", "--quiet", "--depth", "1", "--filter=blob:none",
            UPSTREAM, str(upstream)
        ], check=True)
        subprocess.run([
            "git", "-C", str(upstream), "fetch", "--quiet", "--depth", "1",
            "origin", COMMIT
        ], check=True)
        subprocess.run([
            "git", "-C", str(upstream), "checkout", "--quiet", "--detach", COMMIT
        ], check=True)
        acquire(upstream)


if __name__ == "__main__":
    main()
