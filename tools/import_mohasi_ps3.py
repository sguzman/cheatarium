#!/usr/bin/env python3
"""Acquire every pinned Mohasi PS3 editable/compiled source and its notices.

Preserve exact source bytes and Git blob identity. This is a repackaged,
Artemis-derived PS3 collection: its game serials, reported votes, and compiled
formats are not newly discovered cheat effects or empirical verification.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
ID = "mohasi-ps3-compiled"
URL = "https://github.com/mohasi/game-cheats"
COMMIT = "08f2c50c8baefe52dadcb4920489112cd39596c6"
ARCHIVE = ROOT / "archive" / ID
MANIFEST = ROOT / "sources" / f"{ID}.json"
EXPECTED_COMPILED = 4343
EXPECTED_SOURCE = 13343
EXPECTED_ORIGINALS = EXPECTED_COMPILED + EXPECTED_SOURCE + 2
MAX_FILE_BYTES = 300_000


def git_blob(data: bytes) -> str:
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()


def selected_paths(paths: list[str]) -> tuple[list[str], dict[str, int]]:
    counts = {"compiled": 0, "source": 0}
    selected = []
    seen = set()
    for path in paths:
        parts = PurePosixPath(path).parts
        if (not parts or path.startswith("/") or "\\" in path
                or "\x00" in path or any(x in (".", "..") for x in parts)
                or path in seen):
            raise ValueError(f"Unsafe or duplicate original path: {path!r}")
        seen.add(path)
        if path in ("LICENSE", "README.md"):
            selected.append(path)
            continue
        if (len(parts) != 2 or parts[0] not in counts
                or not parts[1].endswith(".txt")
                or not parts[1][:-4].isalnum()
                or not parts[1][:-4].isascii()):
            continue
        selected.append(path)
        counts[parts[0]] += 1
    if (counts != {"compiled": EXPECTED_COMPILED, "source": EXPECTED_SOURCE}
            or len(selected) != EXPECTED_ORIGINALS):
        raise RuntimeError(f"Pinned PS3 collection inventory changed: {counts}, {len(selected)}")
    return sorted(selected), counts


def acquire(upstream: Path) -> None:
    head = subprocess.check_output(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True
    ).strip()
    if head != COMMIT:
        raise RuntimeError(f"Wrong source revision: {head}")
    path_bytes = subprocess.check_output([
        "git", "-C", str(upstream), "ls-tree", "-r", "-z", "--name-only", "HEAD"
    ])
    paths = [x.decode("utf-8", "strict") for x in path_bytes.split(b"\0") if x]
    chosen, counts = selected_paths(paths)
    previous = {}
    if MANIFEST.exists():
        old = json.loads(MANIFEST.read_text("utf-8"))
        if old["snapshot_commit"] != COMMIT or old["id"] != ID:
            raise RuntimeError("Existing source identity changed")
        previous = {x["upstream_path"]: x for x in old["files"]}
    rows = []
    newly_added = 0
    for path in chosen:
        src = upstream / path
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Missing/non-regular source file: {path}")
        raw = src.read_bytes()
        if not raw or len(raw) > MAX_FILE_BYTES:
            raise RuntimeError(f"Empty or unexpected file size: {path}")
        row = {
            "upstream_path": path,
            "archive_path": f"archive/{ID}/{path}",
            "git_blob_sha": git_blob(raw),
        }
        if path in previous and previous[path] != row:
            raise RuntimeError(f"Already acquired original changed: {path}")
        dest = ARCHIVE / path
        if dest.exists():
            if dest.read_bytes() != raw:
                raise RuntimeError(f"Archive byte mismatch: {path}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest)
            newly_added += 1
        rows.append(row)
    if set(previous) != set(chosen) and previous:
        raise RuntimeError("Previously preserved upstream paths changed")
    manifest = {
        "schema_version": 1,
        "id": ID,
        "repository": URL,
        "snapshot_commit": COMMIT,
        "license": "MIT (compiled/source database grant; original Artemis cheat authors' rights separately credited)",
        "archive_prefix": f"archive/{ID}/",
        "scope": "Full pinned compiled and source PS3 text collections plus original MIT LICENSE and README; excludes software scripts and votes",
        "rights_note": (
            "Derived from already archived Artemis PS3 data, not an independent "
            "source of discovered gameplay effects. Preserve every original author "
            "credit, source name, score, alias and MIT license byte-identically. "
            "Compiled files correspond to source aliases/variants: counts are file "
            "paths, not unique games. Upstream community scoring is reported "
            "feedback, not gameplay verification by Cheatarium. Independently "
            "authored historical cheat rights may vary."
        ),
        "original_file_kinds": counts,
        "source_cheat_files": EXPECTED_COMPILED,
        "files": rows,
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", "utf-8")
    print(json.dumps({
        "pinned_revision": COMMIT,
        "compiled_ps3_originals": counts["compiled"],
        "editable_ps3_originals_including_aliases": counts["source"],
        "notices": 2, "total_original_files": len(rows),
        "newly_preserved_files": newly_added,
    }, indent=2))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--checked-out-upstream", type=Path)
    args = ap.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-mohasi-ps3-") as tmp:
        source = Path(tmp) / "original"
        subprocess.run(["git", "init", "--quiet", str(source)], check=True)
        subprocess.run(["git", "-C", str(source), "remote", "add", "origin", URL + ".git"], check=True)
        subprocess.run(["git", "-C", str(source), "fetch", "--quiet", "--depth=1", "origin", COMMIT], check=True)
        subprocess.run(["git", "-C", str(source), "checkout", "--quiet", "--detach", "FETCH_HEAD"], check=True)
        acquire(source)


if __name__ == "__main__":
    main()
