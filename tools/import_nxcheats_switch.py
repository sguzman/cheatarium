#!/usr/bin/env python3
"""Bulk-preserve source-native Nintendo Switch text cheats from pinned NXCheats.

Preserves the entire eligible source-text surface (including historical _old
variations, code documentation, native patch companions and the MIT notice).
Excludes media and prepacked 7z/zip distribution copies. No code is executed.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from collections import Counter
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
ID = "bad1dea-nxcheats-switch"
URL = "https://github.com/bad1dea/NXCheats"
COMMIT = "047cea8d304c3a6f6697e2f60a6c17b42dc96481"
ARCHIVE = ROOT / "archive" / ID
MANIFEST = ROOT / "sources" / f"{ID}.json"
EXTENSIONS = (".txt", ".txt.noips", ".nxc", ".ips", ".md")
EXPECTED_EXTENSIONS = {
    ".txt": 3008,
    ".txt.noips": 9,
    ".nxc": 23,
    ".ips": 9,
    ".md": 105,
}
EXPECTED_NATIVE_CHEAT_FILES = 2992
EXPECTED_ORIGINALS = sum(EXPECTED_EXTENSIONS.values()) + 1  # LICENSE
MAX_ORIGINAL_BYTES = 2_000_000


def git_blob(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def git_paths(upstream: Path) -> list[str]:
    raw = subprocess.check_output(
        ["git", "-C", str(upstream), "ls-tree", "-r", "-z", "--name-only", "HEAD"]
    )
    return [part.decode("utf-8", "strict") for part in raw.split(b"\0") if part]


def extension(path: str) -> str | None:
    return next((ext for ext in EXTENSIONS if path.endswith(ext)), None)


def is_native_cheat(path: str) -> bool:
    parts = path.split("/")
    filename = parts[-1]
    return (
        "cheats" in parts[:-1]
        and filename.lower().endswith(".txt")
        and len(filename) == 20
        and all(ch in "0123456789abcdefABCDEF" for ch in filename[:-4])
    )


def selected_paths(all_paths: list[str]) -> tuple[list[str], dict[str, int], int]:
    selected = []
    seen = set()
    counts = Counter()
    native_count = 0
    for path in all_paths:
        parts = PurePosixPath(path).parts
        if (not parts or path.startswith("/") or "\\\\" in path or
                any(p in (".", "..") for p in parts) or "\0" in path):
            raise RuntimeError(f"Unsafe upstream path: {path!r}")
        if path in seen:
            raise RuntimeError(f"Repeated Git source path: {path!r}")
        seen.add(path)
        ext = extension(path)
        if path == "LICENSE":
            selected.append(path)
        elif ext:
            # Only original text, native code text and accompanying native IPS.
            # No .7z, .zip, images, game images, keys or binaries.
            selected.append(path)
            counts[ext] += 1
            native_count += int(is_native_cheat(path))
    if dict(counts) != EXPECTED_EXTENSIONS or native_count != EXPECTED_NATIVE_CHEAT_FILES:
        raise RuntimeError(
            f"Pinned source inventory mismatch: {dict(counts)}, native={native_count}"
        )
    if len(selected) != EXPECTED_ORIGINALS:
        raise RuntimeError(f"Pinned source expected {EXPECTED_ORIGINALS}, got {len(selected)}")
    return sorted(selected), dict(sorted(counts.items())), native_count


def acquire(upstream: Path) -> None:
    head = subprocess.check_output(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True
    ).strip()
    if head != COMMIT:
        raise RuntimeError(f"Expected pinned upstream revision {COMMIT}, got {head}")
    paths, counts, native_count = selected_paths(git_paths(upstream))
    previous = {}
    if MANIFEST.exists():
        before = json.loads(MANIFEST.read_text("utf-8"))
        if before["snapshot_commit"] != COMMIT:
            raise RuntimeError("Existing source manifest has a different pinned revision")
        previous = {f["upstream_path"]: f for f in before["files"]}
    records = []
    created = 0
    for path in paths:
        src = upstream / path
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Missing/non-regular original source: {path}")
        raw = src.read_bytes()
        if not raw or len(raw) > MAX_ORIGINAL_BYTES:
            raise RuntimeError(f"Empty or unexpectedly large source: {path}")
        row = {
            "upstream_path": path,
            "archive_path": f"archive/{ID}/{path}",
            "git_blob_sha": git_blob(raw),
        }
        if path in previous and row != previous[path]:
            raise RuntimeError(f"Pinned source provenance changed: {path}")
        dest = ARCHIVE / path
        if dest.exists():
            if dest.read_bytes() != raw:
                raise RuntimeError(f"Archived original bytes changed: {path}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest)
            created += 1
        records.append(row)
    if set(previous) - set(paths):
        raise RuntimeError("Previously acquired originals have been dropped")
    manifest = {
        "schema_version": 1,
        "id": ID,
        "repository": URL,
        "snapshot_commit": COMMIT,
        "license": "MIT (repository source code grant; independently authored/derived cheat rights may vary)",
        "archive_prefix": f"archive/{ID}/",
        "scope": "Complete original text cheat and source documentation subset, including legacy per-emulator cheat variants and native patch companions; no binaries/archives or images",
        "rights_note": (
            "Preserves the original LICENSE, all source .md and source-native .txt, .nxc, "
            ".txt.noips and .ips files, without altering bytes. Excludes packaging archives "
            "and media. README describes cheats the maintainer made or worked on, not an "
            "independent permission grant from every possible third-party creator. "
            "Historical single-effect files and different revisions are not independent "
            "game titles, code effects, or verified build compatibility."
        ),
        "original_file_kinds": counts,
        "source_native_cheat_txt_files": native_count,
        "source_cheat_files": native_count,
        "files": records,
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", "utf-8")
    print(json.dumps({
        "revision": COMMIT, "source_native_cheat_text_files": native_count,
        "preserved_original_files_including_notices": len(records),
        "created_originals": created, "file_kinds": counts
    }, indent=2))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path)
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-nxcheats-") as tmp:
        folder = Path(tmp) / "NXCheats"
        subprocess.run(["git", "clone", "--quiet", "--filter=blob:none",
                        "--depth=1", URL + ".git", str(folder)], check=True)
        subprocess.run(["git", "-C", str(folder), "fetch", "--quiet",
                        "--depth=1", "origin", COMMIT], check=True)
        subprocess.run(["git", "-C", str(folder), "checkout", "--quiet",
                        "--detach", COMMIT], check=True)
        acquire(folder)


if __name__ == "__main__":
    main()
