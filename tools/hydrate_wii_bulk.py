#!/usr/bin/env python3
"""Private, pinned-source Wii Gecko-code hydration with offline integrity checks.

Downloads native Wii Gecko code sources only on --fetch. Never edits or checks
out any game image, runs cheat code, or writes third-party source code into
Cheatarium's public Git tree. Native upstream region/revision filenames survive.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import tempfile

from hydrate_nintendo_bulk import (
    ROOT, fetch_blob, git_blob_sha, safe_components, validate_blob,
)

SOURCE_IDS = (
    "wii-accf-gecko-native",
    "wii-metroid-prime-gecko-native",
    "wii-metroid-other-m-gecko-native",
    "wii-taiko-gecko-native",
)
COUNTS = (9, 12, 3, 5)
FORMAT = "cheatarium-private-wii-gecko-originals-v1"


def default_destination():
    return Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))) / "cheatarium/wii-originals"


def pinned_sources(root=ROOT):
    sources = []
    for sid, count in zip(SOURCE_IDS, COUNTS):
        source = json.loads((root / "sources/external-bulk" / (sid + ".json")).read_text("utf-8"))
        if (source.get("schema_version") != 1
                or source.get("id") != sid
                or source.get("platform") != "wii"
                or source.get("rights_status") != "link_only_rights_review"
                or source.get("file_counts") != {"cheat": count}
                or not isinstance(source.get("files"), list)
                or len(source["files"]) != count):
            raise ValueError("Unknown or rights-changing pinned Wii source")
        previous = ""
        for row in source["files"]:
            if (not isinstance(row, list) or len(row) != 3 or
                    not isinstance(row[0], str) or row[0] <= previous):
                raise ValueError("Invalid source-native Wii path order")
            safe_components(row[0])
            suffix = Path(row[0]).suffix.lower()
            if suffix not in {".ini", ".txt"}:
                raise ValueError("Unexpected executable format in original Wii source")
            previous = row[0]
        sources.append(source)
    return sources


def expected_files(sources):
    files = {}
    for source in sources:
        for path, sha, size in source["files"]:
            relative = safe_components("wii/" + source["id"] + "/" + path)
            if relative in files:
                raise ValueError("Original Wii source path collision")
            files[relative] = {
                "source_id": source["id"],
                "original_path": path,
                "source_git_blob_sha1": sha,
                "bytes": size,
            }
    if len(files) != sum(COUNTS):
        raise ValueError("Wii native source inventory missing files")
    return files


def build_into(stage, sources, fetch):
    known = expected_files(sources)
    records = []
    for source in sources:
        for original in source["files"]:
            raw = fetch(source, original)
            validate_blob(original, raw)
            relative = "wii/" + source["id"] + "/" + original[0]
            target = stage / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(raw)
            records.append({
                "path": relative,
                **known[relative],
                "sha256": hashlib.sha256(raw).hexdigest(),
            })
    records.sort(key=lambda item: item["path"])
    receipt = {
        "schema_version": 1,
        "format": FORMAT,
        "platform": "wii",
        "private_local_only": True,
        "rights_to_redistribute_original_code_bodies": False,
        "gameplay_or_rom_revision_verified": False,
        "source_count": len(SOURCE_IDS),
        "wii_original_gecko_source_files": len(records),
        "source_ids": list(SOURCE_IDS),
        "records": records,
    }
    (stage / "manifest.json").write_text(json.dumps(receipt, indent=2) + "\n", "utf-8")
    return receipt


def check(destination, root=ROOT):
    sources = pinned_sources(root)
    expected = expected_files(sources)
    manifest = json.loads((destination / "manifest.json").read_text("utf-8"))
    if (manifest.get("schema_version") != 1
            or manifest.get("format") != FORMAT
            or manifest.get("platform") != "wii"
            or manifest.get("private_local_only") is not True
            or manifest.get("rights_to_redistribute_original_code_bodies") is not False
            or manifest.get("gameplay_or_rom_revision_verified") is not False
            or manifest.get("source_count") != 4
            or manifest.get("wii_original_gecko_source_files") != 29
            or manifest.get("source_ids") != list(SOURCE_IDS)
            or not isinstance(manifest.get("records"), list)
            or len(manifest["records"]) != 29):
        raise ValueError("Private Wii cache metadata invalid")
    seen = set()
    for record in manifest["records"]:
        path = safe_components(record.get("path"))
        if path not in expected or path in seen:
            raise ValueError("Cached Wii file absent from pinned original manifests")
        seen.add(path)
        if any(record.get(key) != expected[path][key]
               for key in ("source_id", "original_path", "source_git_blob_sha1", "bytes")):
            raise ValueError("Cached Wii file provenance differs from pinned Git original")
        target = destination / path
        if target.is_symlink() or not target.is_file():
            raise ValueError("Missing or linked original Wii source file")
        raw = target.read_bytes()
        validate_blob([expected[path]["original_path"], expected[path]["source_git_blob_sha1"],
                       expected[path]["bytes"]], raw)
        if hashlib.sha256(raw).hexdigest() != record.get("sha256"):
            raise ValueError("Original Wii source no longer matches local SHA-256")
    if seen != set(expected):
        raise ValueError("Private Wii cache incomplete")
    return manifest


def hydrate(destination, fetch=fetch_blob, root=ROOT):
    sources = pinned_sources(root)
    destination = destination.expanduser().resolve()
    if destination == root.resolve() or root.resolve() in destination.parents:
        raise ValueError("Refusing to write third-party Wii Gecko cheat data inside public repository")
    if destination.exists():
        return check(destination, root)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".cheatarium-wii-stage-", dir=destination.parent) as temp:
        stage = Path(temp)
        build_into(stage, sources, fetch)
        check(stage, root)
        if destination.exists():
            raise FileExistsError("Another process already hydrated the Wii source cache")
        stage.rename(destination)
    return check(destination, root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--fetch", action="store_true", help="Hydrate source files into private local cache")
    action.add_argument("--check", action="store_true", help="Verify the existing local cache offline")
    parser.add_argument("--destination", type=Path, default=default_destination())
    args = parser.parse_args()
    manifest = hydrate(args.destination) if args.fetch else check(args.destination.expanduser().resolve())
    print(json.dumps({
        "destination": str(args.destination.expanduser().resolve()),
        "wii_original_gecko_files": manifest["wii_original_gecko_source_files"],
        "source_catalogs": manifest["source_count"],
        "rights_to_redistribute": False,
        "gameplay_verified": False,
        "private_local_only": True,
    }, indent=2))


if __name__ == "__main__":
    main()
