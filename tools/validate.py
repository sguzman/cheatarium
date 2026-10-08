#!/usr/bin/env python3
"""Validate Cheatarium's source inventory, integrity, and curated records. No dependencies."""
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
failures = []

def error(message):
    failures.append(message)

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
    sid = data.get("id")
    if not sid or sid in sources:
        error(f"{manifest}: empty or duplicate source ID")
        continue
    sources[sid] = data
    if not data.get("snapshot_commit") or not data.get("license"):
        error(f"{manifest}: missing snapshot or license")
    for item in data.get("files", []):
        relative = item.get("archive_path", "")
        upstream = item.get("upstream_path", "")
        key = (sid, relative)
        if not relative.startswith(f"archive/{sid}/") or not upstream or key in inventory:
            error(f"{manifest}: malformed/duplicate file record: {relative}")
            continue
        inventory.add(key)
        expected_relative = f"archive/{sid}/{upstream}"
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
            count = re.search(r"(?m)^cheats\s*=\s*(\d+)", text)
            codes = re.findall(r"(?m)^cheat\d+_code\s*=", text)
            if count and int(count.group(1)) != len(codes):
                error(f"{relative}: .cht cheat count mismatch ({count.group(1)} declared, {len(codes)} codes)")

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

for path in (ROOT / "archive").rglob("*"):
    if path.is_file() and not any(path == ROOT / rel for _, rel in inventory):
        error(f"{path.relative_to(ROOT)}: archived file omitted from source inventories")

if failures:
    for message in failures:
        print("ERROR:", message, file=sys.stderr)
    raise SystemExit(f"{len(failures)} problem(s) found")
print(f"OK: {len(platforms)} platforms, {len(sources)} sources, {archive_count} archived files, "
      f"{curated_count} curated games, {cheat_count} curated effects")
