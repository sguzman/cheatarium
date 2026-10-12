#!/usr/bin/env python3
"""Preserve only non-overlapping source blobs from a pinned CC-BY-SA Libretro fork.

Source files and notices remain byte-for-byte unchanged. The base Libretro
collection remains authoritative for all shared Git blob contents; derived or
historically re-published cheats are not newly verified gameplay effects.
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
SOURCE_ID = "bithex-libretro-delta"
SOURCE_REPO = "https://github.com/BitHeX0R/Cheat-database"
SOURCE_REV = "837bc5b7b4b872f5b48c6b2e683819ec02db23b8"
BASE_REV = "fbeefcb46c2e1b20a7e2945f34a694a41b2d6f90"
MANIFEST = ROOT / "sources" / (SOURCE_ID + ".json")
DEST = ROOT / "archive" / SOURCE_ID
FORK_CHEATS = 28278
BASE_CHEATS = 27915
EXPECTED_NEW = 1125
EXPECTED_DISTINCT = 1113
EXPECTED_NOTICES = ("LICENSE", "README.md")
MAX_FILE_BYTES = 200_000


def blob(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()


def tree_rows(git_output: bytes) -> list[tuple[str, str]]:
    result = []
    for part in git_output.split(b"\0"):
        if not part:
            continue
        header, sep, path = part.partition(b"\t")
        meta = header.decode("ascii").split(" ")
        if not sep or len(meta) != 3 or meta[1] != "blob" or meta[0] not in ("100644", "100755"):
            raise ValueError("Malformed or unsupported upstream Git tree entry")
        decoded = path.decode("utf-8", "strict")
        if not decoded or decoded.startswith("/") or "\\" in decoded or "\x00" in decoded:
            raise ValueError("Unsafe upstream Git path")
        if any(p in ("", ".", "..") for p in decoded.split("/")):
            raise ValueError("Traversal or non-normalized Git path")
        if not re.fullmatch(r"[a-f0-9]{40}", meta[2]):
            raise ValueError("Malformed Git blob SHA")
        result.append((decoded, meta[2]))
    if len({p for p, _ in result}) != len(result):
        raise ValueError("Duplicate upstream Git path")
    return result


def selected_delta(rows: list[tuple[str, str]], baseline: set[str]) -> list[tuple[str, str]]:
    return sorted(
        ((path, sha) for path, sha in rows
         if path.startswith("cht/") and path.endswith(".cht") and sha not in baseline),
        key=lambda row: row[0],
    )


def acquire(checkout: Path) -> None:
    revision = subprocess.check_output(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True
    ).strip()
    if revision != SOURCE_REV:
        raise RuntimeError(f"Upstream fork is not pinned at {SOURCE_REV}: {revision}")
    base = json.loads((ROOT / "sources/libretro-database.json").read_text("utf-8"))
    if base.get("id") != "libretro-database" or base.get("snapshot_commit") != BASE_REV:
        raise RuntimeError("Pinned Libretro baseline has changed; re-audit before import")
    base_rows = base.get("files", [])
    if len(base_rows) != BASE_CHEATS or any(
        not x["upstream_path"].startswith("cht/") for x in base_rows
    ):
        raise RuntimeError("Unexpected Libretro source inventory")
    prior_blobs = {row["git_blob_sha"] for row in base_rows}
    data = subprocess.check_output([
        "git", "-C", str(checkout), "ls-tree", "-r", "-z", "HEAD"
    ])
    rows = tree_rows(data)
    all_cheats = [p for p, _ in rows if p.startswith("cht/") and p.endswith(".cht")]
    selected = selected_delta(rows, prior_blobs)
    if (len(all_cheats) != FORK_CHEATS or len(selected) != EXPECTED_NEW
            or len({sha for _, sha in selected}) != EXPECTED_DISTINCT):
        raise RuntimeError(
            f"Pinned delta changed: upstream={len(all_cheats)} "
            f"new={len(selected)} distinct={len({x[1] for x in selected})}"
        )
    by_path = dict(rows)
    if any(path not in by_path for path in EXPECTED_NOTICES):
        raise RuntimeError("Upstream CC BY-SA license or README missing")
    selected = [(p, by_path[p]) for p in EXPECTED_NOTICES] + selected
    previous = {}
    if MANIFEST.is_file():
        prior = json.loads(MANIFEST.read_text("utf-8"))
        if prior.get("id") != SOURCE_ID or prior.get("snapshot_commit") != SOURCE_REV:
            raise RuntimeError("Previously preserved source identity changed")
        previous = {x["upstream_path"]: x for x in prior["files"]}
    records = []
    created = 0
    for path, expected_sha in selected:
        src = checkout / path
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Missing/non-regular original file: {path}")
        raw = src.read_bytes()
        if not raw or len(raw) > MAX_FILE_BYTES or blob(raw) != expected_sha:
            raise RuntimeError(f"Upstream original bytes fail pinned Git blob check: {path}")
        row = {
            "upstream_path": path,
            "archive_path": f"archive/{SOURCE_ID}/{path}",
            "git_blob_sha": expected_sha,
        }
        if path in previous and row != previous[path]:
            raise RuntimeError(f"Previously imported source path changed: {path}")
        dest = DEST / path
        if dest.exists():
            if dest.read_bytes() != raw:
                raise RuntimeError(f"Local archive differs from upstream original: {path}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest)
            created += 1
        records.append(row)
    if previous and set(previous) != {p for p, _ in selected}:
        raise RuntimeError("Earlier source files dropped from unchanged revision")
    source = {
        "schema_version": 1,
        "id": SOURCE_ID,
        "repository": SOURCE_REPO,
        "snapshot_commit": SOURCE_REV,
        "baseline_source": "libretro-database",
        "baseline_snapshot_commit": BASE_REV,
        "license": "CC-BY-SA-4.0 (upstream LICENSE; historic cheat authors' rights may vary)",
        "archive_prefix": f"archive/{SOURCE_ID}/",
        "scope": "Every pinned .cht occurrence whose exact Git blob SHA is absent from the complete pinned original Libretro cheat inventory, plus unchanged original LICENSE and README",
        "rights_note": (
            "Fork of Libretro Database; CC BY-SA 4.0 attribution and original notices "
            "are preserved. The 1,125 filename occurrences represent only 1,113 "
            "distinct blobs absent from pinned Libretro; no verified new game "
            "identities, unique gameplay effects, build compatibility or executable "
            "code behavior are asserted. Other original historical cheat authors "
            "may hold independent rights."
        ),
        "source_cheat_files": EXPECTED_NEW,
        "distinct_new_git_blobs": EXPECTED_DISTINCT,
        "original_file_kinds": {"cheat": EXPECTED_NEW, "notices": 2},
        "files": records,
    }
    MANIFEST.write_text(json.dumps(source, indent=2, ensure_ascii=False) + "\n", "utf-8")
    print(json.dumps({
        "source_revision": SOURCE_REV, "originals": len(records),
        "novel_cheat_file_occurrences": EXPECTED_NEW,
        "distinct_new_git_blobs": EXPECTED_DISTINCT,
        "newly_copied_originals": created,
    }, indent=2))


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--checked-out-upstream", type=Path)
    args = p.parse_args()
    if args.checked_out_upstream is not None:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-bithex-") as tmp:
        folder = Path(tmp) / "upstream"
        subprocess.run([
            "git", "clone", "--quiet", "--filter=blob:none", "--depth=1",
            "--sparse", "--no-checkout", SOURCE_REPO + ".git", str(folder)
        ], check=True)
        subprocess.run([
            "git", "-C", str(folder), "fetch", "--quiet",
            "--depth=1", "origin", SOURCE_REV
        ], check=True)
        subprocess.run([
            "git", "-C", str(folder), "sparse-checkout", "set", "--cone", "cht"
        ], check=True)
        subprocess.run([
            "git", "-C", str(folder), "checkout", "--quiet", "--detach", SOURCE_REV
        ], check=True)
        acquire(folder)


if __name__ == "__main__":
    main()
