#!/usr/bin/env python3
"""Import pinned Libretro cheat files verbatim and record byte-level provenance.

Requires git and network access. Does not commit, push, or modify upstream.
"""
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE_FILE = ROOT / "sources/libretro-database.json"
SOURCE_URL = "https://github.com/libretro/libretro-database.git"
def load_systems():
    mapping = json.loads((ROOT / "platforms/libretro-mapping.json").read_text(encoding="utf-8"))
    if mapping.get("schema_version") != 1:
        raise RuntimeError("Unsupported Libretro mapping schema")
    systems = {}
    directories = set()
    for row in mapping["systems"]:
        key, directory = row["import_id"], row["source_directory"]
        if key in systems or directory in directories:
            raise RuntimeError(f"Duplicate Libretro mapping: {key} / {directory}")
        systems[key] = directory
        directories.add(directory)
    return systems

SYSTEMS = load_systems()

def run(*args):
    return subprocess.check_output(args, text=True).strip()

def blob_sha(content):
    return hashlib.sha1(f"blob {len(content)}\0".encode("ascii") + content).hexdigest()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--systems", nargs="+", choices=sorted(SYSTEMS), help="One or more mapped source-collection IDs")
    parser.add_argument("--all-consoles", action="store_true", help="Import every mapped console/handheld collection")
    parser.add_argument("--limit", type=int, default=0, help="Max files per system; 0 imports all")
    args = parser.parse_args()
    if args.limit < 0:
        parser.error("--limit must be nonnegative")
    if args.all_consoles and args.systems:
        parser.error("Choose --systems or --all-consoles, not both")
    selected = sorted(SYSTEMS) if args.all_consoles else (args.systems or ["nes", "snes"])
    data = json.loads(SOURCE_FILE.read_text(encoding="utf-8"))
    revision = data["snapshot_commit"]
    items = {x["upstream_path"]: x for x in data["files"]}
    requested = [f"cht/{SYSTEMS[s]}" for s in dict.fromkeys(selected)]
    imported = 0
    with tempfile.TemporaryDirectory(prefix="cheatarium-libretro-") as folder:
        source = Path(folder) / "upstream"
        print("Cloning sparse Libretro git metadata...")
        subprocess.run(["git", "clone", "--quiet", "--filter=blob:none", "--depth=1",
                        "--sparse", "--no-checkout", SOURCE_URL, str(source)], check=True)
        subprocess.run(["git", "-C", str(source), "fetch", "--quiet", "--depth=1",
                        "origin", revision], check=True)
        subprocess.run(["git", "-C", str(source), "checkout", "--quiet", "--detach", revision], check=True)
        subprocess.run(["git", "-C", str(source), "sparse-checkout", "set", "--cone", *requested], check=True)
        actual = run("git", "-C", str(source), "rev-parse", "HEAD")
        if actual != revision:
            raise RuntimeError(f"Expected {revision}, checked out {actual}")
        for directory in requested:
            files = sorted((source / directory).rglob("*.cht"))
            if not files:
                raise RuntimeError(f"No .cht files found for {directory}")
            if args.limit:
                files = files[:args.limit]
            for src in files:
                upstream_path = src.relative_to(source).as_posix()
                archive_path = "archive/libretro/" + upstream_path
                dest = ROOT / archive_path
                raw = src.read_bytes()
                sha = blob_sha(raw)
                old = items.get(upstream_path)
                if old and (old["git_blob_sha"] != sha or old["archive_path"] != archive_path):
                    raise RuntimeError(f"Provenance conflict for {upstream_path}")
                if dest.exists() and dest.read_bytes() != raw:
                    raise RuntimeError(f"Local archive differs from pinned upstream: {dest}")
                if not dest.exists():
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(src, dest)
                    imported += 1
                items[upstream_path] = {
                    "upstream_path": upstream_path,
                    "archive_path": archive_path,
                    "git_blob_sha": sha,
                }
            print(f"{directory}: indexed {len(files)} original cheats")
    data["files"] = [items[key] for key in sorted(items)]
    SOURCE_FILE.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"Import complete. New files: {imported}; provenance entries: {len(items)}")

if __name__ == "__main__":
    main()
