#!/usr/bin/env python3
"""Materialize pinned GameCube/Wii U cheat originals into a PRIVATE local cache.

No ROMs, emulator operations, or third-party cheat bodies are added to Git.
Source-level Git blob SHA-1 and GameCube ZIP member inventory are verified.
Only run with --fetch to obtain original source files into a user-owned path.
--check is strictly offline and verifies the extracted files against the
local cache manifest.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import tempfile
import urllib.parse
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE_IDS = ("leumas-gamecube-wiird-zips", "katsumaki-wiiu-xml", "ucodes-wiiu-json")
REPORT = ROOT / "research/nintendo-gamecube-wiird-member-claims-v1.json"
FORMAT = "cheatarium-local-nintendo-original-cheat-cache-v1"
REGIONS = {"NTSC-J": 8, "NTSC-U": 138, "PAL": 229}
LIMIT = 100_000_000
MEMBER_LIMIT = 5_000_000


def default_destination():
    home = Path(os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share")))
    return home / "cheatarium" / "nintendo-originals"


def safe_components(relative):
    if (not isinstance(relative, str) or not relative or
            "\x00" in relative or "\\" in relative or
            relative.startswith("/") or
            any(p in ("", ".", "..") for p in relative.split("/"))):
        raise ValueError("Unsafe original source path")
    return relative


def git_blob_sha(data):
    return hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()


def validate_blob(original, data):
    path, sha, size = original
    if (not isinstance(sha, str) or not re.fullmatch(r"[a-f0-9]{40}", sha) or
            not isinstance(size, int) or isinstance(size, bool) or
            size < 0 or size > LIMIT or len(data) != size or
            git_blob_sha(data) != sha):
        raise ValueError("Upstream original Git blob checksum or size mismatch: " + path)


def fetch_blob(source, original):
    path, _sha, size = original
    safe_components(path)
    repo = source["repository"].removeprefix("https://github.com/")
    rev = source["snapshot_commit"]
    if (not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo)
            or not re.fullmatch(r"[a-f0-9]{40}", rev)
            or size > LIMIT):
        raise ValueError("Untrusted original revision/repository/size")
    url = "https://raw.githubusercontent.com/" + repo + "/" + rev + "/" + urllib.parse.quote(path, safe="/")
    with urllib.request.urlopen(urllib.request.Request(
            url, headers={"User-Agent": "Cheatarium-local-Nintendo-source-acquisition",
                          "Accept": "application/octet-stream"}), timeout=60) as response:
        raw = response.read(size + 1)
    validate_blob(original, raw)
    return raw


def pinned_sources(root=ROOT):
    sources = []
    for sid in SOURCE_IDS:
        path = root / "sources/external-bulk" / (sid + ".json")
        source = json.loads(path.read_text("utf-8"))
        if (source.get("id") != sid or source.get("rights_status") != "link_only_rights_review"
                or not isinstance(source.get("files"), list)
                or not source["files"]
                or source.get("file_counts") != {"cheat": len(source["files"])}):
            raise ValueError("Untrusted pinned Nintendo source inventory: " + sid)
        for row in source["files"]:
            if not isinstance(row, list) or len(row) != 3:
                raise ValueError("Malformed pinned original file entry")
            safe_components(row[0])
        sources.append(source)
    if [len(s["files"]) for s in sources] != [3, 9, 4]:
        raise ValueError("Nintendo source counts changed without audit")
    doc = json.loads((root / "research/nintendo-gamecube-wiird-member-claims-v1.json").read_text("utf-8"))
    if (doc.get("format") != "cheatarium-nintendo-gamecube-wiird-member-references-v1"
            or doc.get("reference_count") != 375
            or doc.get("file_occurrences_by_region") != REGIONS
            or doc.get("rights_status") != "link_only_rights_review"
            or doc.get("cheat_bodies_included") is not False):
        raise ValueError("GameCube member inventory not verified or not code-free")
    return sources, doc


def add_file(stage, records, relative, raw, sid, path, container_sha):
    relative = safe_components(relative)
    if relative in records:
        raise ValueError("Original cheat cache path collision")
    target = stage / relative
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(raw)
    records[relative] = {
        "path": relative, "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
        "source_id": sid, "original_path_or_zip_member": path,
        "source_container_git_blob_sha1": container_sha,
    }


def build_into(stage, sources, members, fetch):
    records = {}
    source = sources[0]
    expected = {}
    for m in members["records"]:
        region = m["region"]
        if region not in REGIONS:
            raise ValueError("Unexpected GameCube region")
        container = safe_components(m["original_container_path"])
        native = safe_components(m["native_zip_member_path"])
        if not native.lower().endswith(".txt") or m["source_member_uncompressed_bytes"] > MEMBER_LIMIT:
            raise ValueError("Unexpected GameCube original member format/size")
        key = (container, native)
        if key in expected:
            raise ValueError("Duplicate GameCube source member metadata")
        expected[key] = m
    observed = set()
    for row in source["files"]:
        raw = fetch(source, row)
        validate_blob(row, raw)
        with zipfile.ZipFile(io.BytesIO(raw)) as z:
            files = [i for i in z.infolist() if not i.is_dir()]
            if len(files) > 5000 or sum(i.file_size for i in files) > 300_000_000:
                raise ValueError("Untrusted source ZIP decompressed bounds")
            for item in files:
                native = safe_components(item.filename.replace("\\", "/"))
                key = (row[0], native)
                if key not in expected or key in observed or item.file_size != expected[key]["source_member_uncompressed_bytes"]:
                    raise ValueError("GameCube ZIP contains unpinned or changed member")
                if (expected[key]["original_container_git_blob_sha1"] != row[1]
                        or item.file_size > MEMBER_LIMIT):
                    raise ValueError("GameCube ZIP identity or per-member size changed")
                observed.add(key)
                with z.open(item) as f:
                    body = f.read(item.file_size + 1)
                if len(body) != item.file_size:
                    raise ValueError("GameCube source ZIP member length changed")
                add_file(stage, records, "gamecube/" + expected[key]["region"] + "/" + native,
                         body, source["id"], row[0] + "::" + native, row[1])
    if observed != set(expected):
        raise ValueError("Pinned regional GameCube code file inventory is incomplete")

    for source in sources[1:]:
        for row in source["files"]:
            raw = fetch(source, row)
            validate_blob(row, raw)
            add_file(stage, records, "wii-u/" + source["id"] + "/" + row[0],
                     raw, source["id"], row[0], row[1])
    if len(records) != 388:
        raise ValueError("Expected 375 GameCube and 13 Wii U original cheat files")
    index = {
        "schema_version": 1,
        "format": FORMAT,
        "private_local_only": True,
        "public_repository_contains_original_cheat_bodies": False,
        "original_author_redistribution_rights_cleared": False,
        "gameplay_effects_verified": False,
        "original_sha1_verified": True,
        "gamecube_files": 375,
        "wiiu_files": 13,
        "source_ids": list(SOURCE_IDS),
        "records": [records[k] for k in sorted(records)],
    }
    (stage / "manifest.json").write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n", "utf-8")
    return index


def check(destination):
    manifest = json.loads((destination / "manifest.json").read_text("utf-8"))
    if (manifest.get("format") != FORMAT or
            manifest.get("private_local_only") is not True or
            manifest.get("original_author_redistribution_rights_cleared") is not False or
            manifest.get("source_ids") != list(SOURCE_IDS) or
            manifest.get("gamecube_files") != 375 or
            manifest.get("wiiu_files") != 13 or
            len(manifest.get("records", [])) != 388):
        raise ValueError("Local Nintendo source cache manifest invalid or incomplete")
    seen = set()
    for row in manifest["records"]:
        relative = safe_components(row["path"])
        if relative in seen or not relative.startswith(("gamecube/", "wii-u/")):
            raise ValueError("Duplicate or wrong-platform local Nintendo cheat path")
        seen.add(relative)
        file = destination / relative
        if file.is_symlink() or not file.is_file():
            raise ValueError("Cached original is missing or a symlink: " + relative)
        raw = file.read_bytes()
        if len(raw) != row["bytes"] or hashlib.sha256(raw).hexdigest() != row["sha256"]:
            raise ValueError("Local Nintendo original changed: " + relative)
    return manifest


def hydrate(destination, fetch=fetch_blob, root=ROOT):
    sources, members = pinned_sources(root)
    destination = destination.expanduser().resolve()
    # Prevent accidental publication through archive/source directories or
    # a working tree's untracked files. User-owned cache lives elsewhere.
    if destination == root or root.resolve() in destination.parents:
        raise ValueError("Refusing to place rights-unresolved Nintendo originals inside the public repository")
    if destination.exists():
        return check(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".cheatarium-nintendo-stage-", dir=destination.parent) as temp:
        stage = Path(temp)
        build_into(stage, sources, members, fetch)
        check(stage)
        # The final destination is created only after every source and member
        # has passed SHA + size checks, preventing partial visible hydrations.
        if destination.exists():
            raise FileExistsError("Another Nintendo hydration created this cache")
        stage.rename(destination)
    return check(destination)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    mode = p.add_mutually_exclusive_group(required=True)
    mode.add_argument("--fetch", action="store_true", help="Fetch pinned originals into local user cache")
    mode.add_argument("--check", action="store_true", help="Check existing cache fully offline")
    p.add_argument("--destination", type=Path, default=default_destination())
    args = p.parse_args()
    manifest = hydrate(args.destination) if args.fetch else check(args.destination.expanduser().resolve())
    print(json.dumps({
        "destination": str(args.destination.expanduser().resolve()),
        "gamecube_original_text_files": manifest["gamecube_files"],
        "wiiu_original_xml_json_files": manifest["wiiu_files"],
        "verified_original_files": len(manifest["records"]),
        "local_only": True, "no_roms": True,
        "gameplay_verified": False,
        "rights_to_republish": False,
    }, indent=2))


if __name__ == "__main__":
    main()
