#!/usr/bin/env python3
"""Archive the pinned Artemis PS3 native .ncl cheat corpus with original credits.

The source repository declares MIT for its software and accepts community
cheat-file contributions; keep original authors and native text intact.
The root MIT license alone is not proof of rights for every historical code.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = "https://github.com/bucanero/ArtemisPS3.git"
COMMIT = "69b1c537057138d23d63fe9ec3430e3b4f02fd22"
MANIFEST = ROOT / "sources/artemis-ps3.json"
ARCHIVE = ROOT / "archive/artemis-ps3"
EXPECTED_CODES = 2540
NOTICES = {"LICENSE", "README.md", "docs/README.md"}


def blob_sha(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def selected_source_paths(upstream: Path) -> list:
    original = [
        entry.decode("utf-8", errors="surrogateescape")
        for entry in subprocess.check_output(
            ["git", "-C", str(upstream), "ls-tree", "-r", "-z", "--name-only", "HEAD"],
        ).split(b"\0")
        if entry
    ]
    selected = set()
    ncl_paths = []
    for relative in original:
        path = PurePosixPath(relative)
        if path.is_absolute() or not path.parts or ".." in path.parts:
            raise RuntimeError(f"Unsafe upstream path: {relative}")
        if relative in NOTICES:
            selected.add(relative)
        elif len(path.parts) == 3 and path.parts[:2] == ("docs", "codes"):
            if path.suffix.lower() != ".ncl":
                raise RuntimeError(f"Unexpected Artemis cheat source extension: {relative}")
            ncl_paths.append(relative)
            selected.add(relative)
        elif len(path.parts) >= 2 and path.parts[:2] == ("docs", "codes"):
            raise RuntimeError(f"Unexpected nested Artemis cheat path: {relative}")
    if len(ncl_paths) != EXPECTED_CODES:
        raise RuntimeError(f"Expected {EXPECTED_CODES} native .ncl files, found {len(ncl_paths)}")
    if not NOTICES.issubset(selected):
        raise RuntimeError("Missing source license or original credit README")
    return sorted(selected)


def acquire(upstream: Path):
    checked_out = subprocess.check_output(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True
    ).strip()
    if checked_out != COMMIT:
        raise RuntimeError(f"Source revision mismatch: {checked_out} != {COMMIT}")
    selected = selected_source_paths(upstream)
    previous = {}
    if MANIFEST.exists():
        old = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if old.get("snapshot_commit") != COMMIT:
            raise RuntimeError("Refusing to merge distinct Artemis snapshots")
        previous = {entry["upstream_path"]: entry for entry in old["files"]}
    data = []
    new_files = 0
    for relative in selected:
        source = upstream / relative
        if not source.is_file() or source.is_symlink():
            raise RuntimeError(f"Unexpected symlink, missing source or directory: {relative}")
        raw = source.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty original source: {relative}")
        entry = {
            "upstream_path": relative,
            "archive_path": f"archive/artemis-ps3/{relative}",
            "git_blob_sha": blob_sha(raw),
        }
        if relative in previous and previous[relative] != entry:
            raise RuntimeError(f"Conflicting previous source inventory: {relative}")
        target = ARCHIVE / relative
        if target.exists():
            if target.read_bytes() != raw:
                raise RuntimeError(f"Original archive modified: {relative}")
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            new_files += 1
        data.append(entry)
    if set(previous).difference({item["upstream_path"] for item in data}):
        raise RuntimeError("Previously inventoried Artemis originals disappeared")
    manifest = {
        "schema_version": 1,
        "id": "artemis-ps3",
        "repository": "https://github.com/bucanero/ArtemisPS3",
        "snapshot_commit": COMMIT,
        "license": "MIT (upstream repository software license; individual community cheat rights not independently verified)",
        "archive_prefix": "archive/artemis-ps3/",
        "platform": "ps3",
        "native_format": "Artemis NCL",
        "native_cheat_files": EXPECTED_CODES,
        "rights_note": (
            "Artemis root MIT LICENSE and both upstream READMEs copied verbatim. "
            "Many NCL payloads identify their original authors inline. "
            "Third-party/historical contributed code rights may differ from "
            "the encompassing repository license; do not relicense those materials."
        ),
        "files": data,
    }
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "revision": COMMIT,
        "new_files_including_notices": new_files,
        "native_ps3_cheat_files": EXPECTED_CODES,
        "manifest_source_files": len(data),
    }, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path, default=None)
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-artemis-ps3-") as tmp:
        source = Path(tmp) / "ArtemisPS3"
        subprocess.run([
            "git", "clone", "--quiet", "--depth", "1", "--filter=blob:none",
            UPSTREAM, str(source),
        ], check=True)
        subprocess.run([
            "git", "-C", str(source), "fetch", "--quiet", "--depth", "1", "origin", COMMIT,
        ], check=True)
        subprocess.run([
            "git", "-C", str(source), "checkout", "--quiet", "--detach", COMMIT,
        ], check=True)
        acquire(source)


if __name__ == "__main__":
    main()
