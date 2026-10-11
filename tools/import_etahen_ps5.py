#!/usr/bin/env python3
"""Bulk-archive the complete PS5-native etaHEN cheat subset at a pinned Git revision.

Preserve original bytes and Git provenance; do not copy duplicate PS4 collections,
make build-compatibility claims, or modify emulator/game files.
"""
import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = "https://github.com/etaHEN/PS5_Cheats.git"
REPOSITORY = "https://github.com/etaHEN/PS5_Cheats"
COMMIT = "ef295511ea4252474b16325618ffaafd3afb3cbb"
ID = "etahen-ps5"
ARCHIVE = ROOT / "archive" / ID
MANIFEST = ROOT / "sources" / (ID + ".json")
EXPECTED = {"json": 15, "shn": 31, "mc4": 29}
EXPECTED_TITLE_IDS = 56
NOTICES = {".github/README.md", "LICENSE", "json.txt", "shn.txt", "mc4.txt"}
NAME = re.compile(r"^PPSA[0-9]{5}_.+\.(json|shn|mc4)$", re.IGNORECASE)


def original_blob_sha(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def select(root: Path) -> tuple[list[str], dict[str, int], set[str]]:
    names = subprocess.check_output(
        ["git", "-C", str(root), "ls-tree", "-r", "-z", "--name-only", "HEAD"]
    ).split(b"\0")
    wanted = set(NOTICES)
    counts = {key: 0 for key in EXPECTED}
    title_ids = set()
    for encoded in names:
        if not encoded:
            continue
        relative = encoded.decode("utf-8", errors="strict")
        parts = PurePosixPath(relative).parts
        if not parts or ".." in parts or PurePosixPath(relative).is_absolute():
            raise RuntimeError(f"Unsafe upstream path: {relative}")
        if len(parts) != 2 or parts[0] not in EXPECTED:
            continue
        if not NAME.fullmatch(parts[1]):
            continue
        wanted.add(relative)
        counts[parts[0]] += 1
        title_ids.add(parts[1][:9].upper())
    if counts != EXPECTED or len(title_ids) != EXPECTED_TITLE_IDS:
        raise RuntimeError(
            f"Pinned PS5 subset changed: counts={counts}, title_ids={len(title_ids)}"
        )
    for relative in NOTICES:
        if not (root / relative).is_file():
            raise RuntimeError(f"Missing original license/attribution: {relative}")
    return sorted(wanted), counts, title_ids


def acquire(root: Path) -> None:
    head = subprocess.check_output(
        ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
    ).strip()
    if head != COMMIT:
        raise RuntimeError(f"Expected pinned revision {COMMIT}, found {head}")
    paths, counts, title_ids = select(root)
    old = {}
    if MANIFEST.exists():
        prior = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if prior["snapshot_commit"] != COMMIT:
            raise RuntimeError("Existing PS5 source uses a different revision")
        old = {x["upstream_path"]: x for x in prior["files"]}
    records = []
    created = 0
    for relative in paths:
        src = root / relative
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Upstream source must be a regular file: {relative}")
        raw = src.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty upstream file: {relative}")
        record = {
            "upstream_path": relative,
            "archive_path": f"archive/{ID}/{relative}",
            "git_blob_sha": original_blob_sha(raw),
        }
        if relative in old and record != old[relative]:
            raise RuntimeError(f"Pinned source conflict: {relative}")
        dest = ARCHIVE / relative
        if dest.exists():
            if dest.read_bytes() != raw:
                raise RuntimeError(f"Archived original modified: {relative}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest)
            created += 1
        records.append(record)
    if set(old) - {x["upstream_path"] for x in records}:
        raise RuntimeError("Previously imported source was dropped")
    manifest = {
        "schema_version": 1,
        "id": ID,
        "repository": REPOSITORY,
        "snapshot_commit": COMMIT,
        "license": "GPL-3.0-only (repository notice; third-party cheat creator rights separately reserved)",
        "archive_prefix": f"archive/{ID}/",
        "scope": "Entire source-native PPSA PS5 cheat subset from etaHEN's json, shn and mc4 folders; original execution not verified",
        "rights_note": "The original etaHEN GPL-3.0 license, code-author/PS4Trainer credits, format lists and README are preserved. Repository-wide licensing does not prove ownership of all earlier cheats.",
        "native_format_counts": counts,
        "source_ps5_title_ids": len(title_ids),
        "source_cheat_files": sum(counts.values()),
        "files": records,
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "revision": COMMIT, "new_original_files_including_notices": created,
        "source_cheat_files": sum(counts.values()), "source_ps5_title_ids": len(title_ids),
        "by_upstream_directory": counts, "preserved_notices": len(NOTICES),
        "manifest_records": len(records)
    }, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path)
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-etahen-ps5-") as folder:
        upstream = Path(folder) / "PS5_Cheats"
        subprocess.run(
            ["git", "clone", "--quiet", "--filter=blob:none", "--depth=1",
             UPSTREAM, str(upstream)],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(upstream), "fetch", "--quiet", "--depth=1",
             "origin", COMMIT],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(upstream), "checkout", "--quiet", "--detach", COMMIT],
            check=True,
        )
        acquire(upstream)


if __name__ == "__main__":
    main()
