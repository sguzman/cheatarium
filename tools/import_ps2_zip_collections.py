#!/usr/bin/env python3
"""Bulk-import four complete, pinned CheatDevicePS2 console cheat ZIPs.

Preserve original ZIP Git blobs, archive every member byte-for-byte, and retain
exact ZIP member paths and container hashes as provenance. Never execute code.
"""
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
ID = "israpps-cheatdevice-ps2-extras"
UPSTREAM = "https://github.com/israpps/CheatDevicePS2"
REVISION = "77b453ad41dace65789ca4c152c94b19f7f55934"
EXPECTED = {"MASTERCODES.zip": 4014, "NTSC_CHEATS.zip": 1880,
            "PAL_CHEATS.zip": 1349, "WIDESCREEN.zip": 1}
NOTICES = ("LICENSE", "README.md")
MANIFEST = ROOT / "sources" / f"{ID}.json"
ARCHIVE = ROOT / "archive" / ID

def blob_sha(data):
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()

def write_original(relative, raw, old):
    record = {"upstream_path": relative,
              "archive_path": f"archive/{ID}/{relative}",
              "git_blob_sha": blob_sha(raw)}
    if relative in old and any(old[relative].get(key) != value for key, value in record.items()):
        raise RuntimeError(f"Original provenance conflict for {relative}")
    dest = ROOT / record["archive_path"]
    if dest.exists():
        if dest.read_bytes() != raw:
            raise RuntimeError(f"Archived original differs from pinned upstream: {relative}")
        return record, False
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(raw)
    return record, True

def safe_member(name):
    if not name or name.startswith("/") or "\\" in name or "\0" in name:
        raise RuntimeError(f"Unsafe ZIP entry path: {name!r}")
    parts = PurePosixPath(name).parts
    if not parts or ".." in parts:
        raise RuntimeError(f"ZIP traversal path: {name!r}")
    cleaned = tuple(p for p in parts if p not in ("", "."))
    if not cleaned:
        raise RuntimeError(f"Empty ZIP entry path: {name!r}")
    return "/".join(cleaned)

def acquire(root):
    checked = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"],
                                      text=True).strip()
    if checked != REVISION:
        raise RuntimeError(f"Unexpected upstream revision: {checked}")
    old = {}
    if MANIFEST.exists():
        existing = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if existing.get("snapshot_commit") != REVISION:
            raise RuntimeError("Existing ZIP member manifest uses another Git revision")
        old = {r["upstream_path"]: r for r in existing["files"]}
    records = []
    created = 0
    member_total = 0
    observed = {}
    for notice in NOTICES:
        path = root / notice
        if not path.is_file() or path.is_symlink():
            raise RuntimeError(f"Missing original upstream license/README: {notice}")
        record, fresh = write_original(notice, path.read_bytes(), old)
        records.append(record)
        created += int(fresh)
    for name, expected in EXPECTED.items():
        relative = "extra_cheats/" + name
        zip_path = root / relative
        if not zip_path.is_file() or zip_path.is_symlink():
            raise RuntimeError(f"Missing original ZIP: {relative}")
        zip_raw = zip_path.read_bytes()
        zip_record, fresh = write_original(relative, zip_raw, old)
        records.append(zip_record)
        created += int(fresh)
        zip_sha = zip_record["git_blob_sha"]
        zip_members = 0
        unique_paths = set()
        with zipfile.ZipFile(zip_path) as z:
            for info in z.infolist():
                if info.is_dir():
                    continue
                if info.flag_bits & 1 or info.file_size > 20000000:
                    raise RuntimeError(f"Unsafe/encrypted/oversize ZIP member: {info.filename}")
                member_path = safe_member(info.filename)
                relative_member = f"zip-members/{name}/{member_path}"
                if relative_member in unique_paths:
                    raise RuntimeError(f"Duplicate/colliding normalized ZIP member: {relative_member}")
                unique_paths.add(relative_member)
                raw = z.read(info)
                if len(raw) != info.file_size:
                    raise RuntimeError(f"Incomplete ZIP member: {relative_member}")
                row, fresh = write_original(relative_member, raw, old)
                row["source_zip"] = relative
                row["source_zip_git_blob_sha"] = zip_sha
                row["zip_member_path"] = info.filename
                row["zip_member_crc32"] = f"{info.CRC:08x}"
                if relative_member in old and old[relative_member] != row:
                    raise RuntimeError(f"Pinned ZIP member provenance altered: {relative_member}")
                records.append(row)
                created += int(fresh)
                zip_members += 1
                member_total += 1
        if zip_members != expected:
            raise RuntimeError(f"Pinned {name} expected {expected} files, found {zip_members}")
        observed[name] = zip_members
        print(f"{name}: {zip_members} archived source members")
    if set(old) - {r["upstream_path"] for r in records}:
        raise RuntimeError("Previously archived ZIP source member was dropped")
    data = {
        "schema_version": 1,
        "id": ID,
        "repository": UPSTREAM,
        "snapshot_commit": REVISION,
        "license": "GPL-3.0-only (repository license; historic third-party device-code author permissions independently unresolved)",
        "archive_prefix": f"archive/{ID}/",
        "scope": "Complete four source-native CheatDevicePS2 ZIP packs with NTSC/PAL game cheats, mastercodes and widescreen sections",
        "rights_note": "Exact ZIP Git blobs and all extracted native member bytes are archived; member paths/CRC/container Git SHA bind each extracted payload to its pinned original ZIP. Computed Git-blob hashes of extracted members identify their bytes and do not imply those members were separate upstream Git objects. Original independent cheat authors are not presumed to have supplied blanket relicensing.",
        "source_zip_member_counts": observed,
        "source_archive_members": member_total,
        "source_game_cheat_files": EXPECTED["NTSC_CHEATS.zip"] + EXPECTED["PAL_CHEATS.zip"],
        "files": records,
    }
    MANIFEST.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({"revision": REVISION, "ZIP_packs": len(EXPECTED),
                      "source_members": member_total,
                      "NTSC_PAL_game_cheat_files": data["source_game_cheat_files"],
                      "created_archive_entries_including_notices": created,
                      "inventory_records": len(records)}, indent=2))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path)
    args = parser.parse_args()
    if args.checked_out_upstream:
        acquire(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-ps2-zips-") as temp:
        src = Path(temp) / "CheatDevicePS2"
        subprocess.run(["git","clone","--quiet","--filter=blob:none","--depth=1",
                        UPSTREAM+".git",str(src)],check=True)
        subprocess.run(["git","-C",str(src),"fetch","--quiet","--depth=1",
                        "origin",REVISION],check=True)
        subprocess.run(["git","-C",str(src),"checkout","--quiet","--detach",REVISION],check=True)
        acquire(src)
if __name__ == "__main__":
    main()
