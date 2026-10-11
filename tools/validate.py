#!/usr/bin/env python3
"""Validate Cheatarium's source inventory, integrity, and curated records. No dependencies."""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
failures = []
warnings = []

def error(message):
    failures.append(message)

def warning(message):
    warnings.append(message)

def load(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        error(f"{path.relative_to(ROOT)}: {exc}")
        return None

platform_data = load(ROOT / "platforms/platforms.json") or {}
platforms = {p["id"] for p in platform_data.get("platforms", [])}
sources = {}
inventory = set()
archive_count = 0
for manifest in sorted((ROOT / "sources").glob("*.json")):
    data = load(manifest)
    if not data:
        continue
    # Acquisition queues describe third-party sources *not* imported yet.
    # They cannot be treated as source inventories or awarded source IDs.
    if manifest.name == "candidates.json":
        if data.get("schema_version") != 1 or not isinstance(data.get("candidates"), list):
            error(f"{manifest}: malformed acquisition queue")
        else:
            seen = set()
            for candidate in data["candidates"]:
                if not isinstance(candidate, dict):
                    error(f"{manifest}: malformed candidate entry")
                    continue
                cid = candidate.get("id")
                if not isinstance(cid, str) or not cid or cid in seen:
                    error(f"{manifest}: empty or duplicate candidate ID")
                seen.add(cid)
                if not isinstance(candidate.get("repo"), str) or not candidate["repo"].startswith("https://"):
                    error(f"{manifest}: candidate lacks HTTPS source URL")
                if candidate.get("state") not in ("link_only_rights_review", "needs_file_level_review"):
                    error(f"{manifest}: candidate lacks a recognized review state")
        continue
    sid = data.get("id")
    if not sid or sid in sources:
        error(f"{manifest}: empty or duplicate source ID")
        continue
    sources[sid] = data
    if not data.get("snapshot_commit") or not data.get("license"):
        error(f"{manifest}: missing snapshot or license")
    archive_prefix = data.get("archive_prefix", f"archive/{sid}/")
    for item in data.get("files", []):
        relative = item.get("archive_path", "")
        upstream = item.get("upstream_path", "")
        key = (sid, relative)
        if not relative.startswith(archive_prefix) or not upstream or key in inventory:
            error(f"{manifest}: malformed/duplicate file record: {relative}")
            continue
        inventory.add(key)
        expected_relative = archive_prefix + upstream
        if expected_relative != relative:
            error(f"{manifest}: path no longer preserves upstream layout: {relative}")
        path = ROOT / relative
        if not path.is_file():
            error(f"{manifest}: missing archived file {relative}")
            continue
        raw = path.read_bytes()
        git_blob_sha = hashlib.sha1(f"blob {len(raw)}\0".encode() + raw).hexdigest()
        if git_blob_sha != item.get("git_blob_sha"):
            error(f"{relative}: imported bytes differ from declared original Git blob")
        archive_count += 1
        if path.suffix == ".cht":
            text = raw.decode("utf-8", errors="replace")
            count = re.search(r'(?m)^cheats\s*=\s*"?(\d+)"?', text)
            # RetroArch .cht files often use description-only section headings.
            # "cheats" counts indexed entries, not only code-bearing entries.
            entries = set(re.findall(r"(?m)^cheat(\d+)_(?:desc|code|enable)\s*=", text))
            if count and int(count.group(1)) != len(entries):
                warning(f"{relative}: .cht entry count mismatch ({count.group(1)} declared, {len(entries)} indexed entries)")

curated_count = 0
cheat_count = 0
for path in sorted((ROOT / "curated").glob("*/*/cheats.json")):
    doc = load(path)
    if doc is None:
        continue
    curated_count += 1
    if doc.get("schema_version") != 1:
        error(f"{path}: unsupported schema_version")
    parts = path.relative_to(ROOT).parts
    if doc.get("platform") != parts[1] or doc.get("game", {}).get("id") != parts[2]:
        error(f"{path}: game path/identity mismatch")
    if doc.get("platform") not in platforms:
        error(f"{path}: unknown platform")
    editions = [e.get("id") for e in doc.get("editions", [])]
    if not editions or len(editions) != len(set(editions)):
        error(f"{path}: no editions or duplicate edition IDs")
    cheat_ids = [c.get("id") for c in doc.get("cheats", [])]
    if len(cheat_ids) != len(set(cheat_ids)):
        error(f"{path}: duplicate cheat IDs")
    for cheat in doc.get("cheats", []):
        cheat_count += 1
        if not cheat.get("name") or not cheat.get("variants"):
            error(f"{path}: incomplete cheat entry")
        for variant in cheat.get("variants", []):
            if variant.get("edition") not in editions or not variant.get("format") or not variant.get("code"):
                error(f"{path}: invalid variant")
            if variant.get("status") not in ("unverified", "verified", "broken"):
                error(f"{path}: invalid verification status")
            ref = variant.get("source", {})
            if (ref.get("source"), ref.get("archive_path")) not in inventory:
                error(f"{path}: untracked source reference {ref}")

# Verify that each extracted PS2 ZIP member still matches the exact original
# archive payload, including its member path, container blob hash and CRC.
# Member-computed Git hashes are byte IDs, not assertions of upstream loose blobs.
if "israpps-cheatdevice-ps2-extras" in sources:
    import zipfile
    pack = sources["israpps-cheatdevice-ps2-extras"]
    rows = pack["files"]
    lookup = {x["upstream_path"]: x for x in rows}
    opened = {}
    try:
        for row in rows:
            if "source_zip" not in row:
                continue
            zip_rel = row["source_zip"]
            source_zip_row = lookup.get(zip_rel)
            if not source_zip_row:
                error(f"PS2 ZIP member lacks original container record: {row['upstream_path']}")
                continue
            if row["source_zip_git_blob_sha"] != source_zip_row["git_blob_sha"]:
                error(f"PS2 ZIP container SHA mismatch: {row['upstream_path']}")
                continue
            if zip_rel not in opened:
                opened[zip_rel] = zipfile.ZipFile(ROOT / source_zip_row["archive_path"])
            name = row["zip_member_path"]
            try:
                member = opened[zip_rel].getinfo(name)
                if f"{member.CRC:08x}" != row["zip_member_crc32"]:
                    error(f"PS2 ZIP member CRC metadata mismatch: {row['upstream_path']}")
                if opened[zip_rel].read(name) != (ROOT / row["archive_path"]).read_bytes():
                    error(f"PS2 ZIP extracted bytes differ from original: {row['upstream_path']}")
            except (KeyError, OSError, zipfile.BadZipFile) as exc:
                error(f"PS2 ZIP original member unavailable {row['upstream_path']}: {exc}")
    finally:
        for handle in opened.values():
            handle.close()

tracked_archive_paths = {rel for _, rel in inventory}
for path in (ROOT / "archive").rglob("*"):
    if path.is_file() and path.relative_to(ROOT).as_posix() not in tracked_archive_paths:
        error(f"{path.relative_to(ROOT)}: archived file omitted from source inventories")

for message in warnings:
    print("WARN:", message, file=sys.stderr)
if failures:
    for message in failures:
        print("ERROR:", message, file=sys.stderr)
    raise SystemExit(f"{len(failures)} problem(s) found")
print(f"OK ({len(warnings)} upstream format warnings): {len(platforms)} platforms, {len(sources)} sources, {archive_count} archived files, "
      f"{curated_count} curated games, {cheat_count} curated effects")
