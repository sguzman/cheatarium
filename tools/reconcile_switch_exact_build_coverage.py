#!/usr/bin/env python3
"""Reconcile exact-build Switch coverage against source-backed curated codes.

Uses only pinned source-code fingerprints and Cheatarium's archived originals.
No network, emulator interaction, third-party code acquisition or publication.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
REPORT = Path("research/switch-exact-build-coverage-v1.json")
HEX = re.compile(r"^[0-9A-F]{8}$")


def fingerprint(code: str) -> str:
    """Canonical body digest matching the reference report's explicit rule."""
    lines = []
    for original in code.splitlines():
        words = original.upper().split()
        if not words:
            continue
        if not (1 <= len(words) <= 5) or any(not HEX.fullmatch(w) for w in words):
            raise ValueError("Invalid native-hex section (not safe to match)")
        lines.append(" ".join(words))
    if not lines:
        raise ValueError("Empty native-hex section")
    return hashlib.sha256("\n".join(lines).encode("ascii")).hexdigest()


def archived_candidates(root: Path, game: dict) -> list[dict]:
    rel = Path(game["curated_file"])
    if rel.is_absolute() or ".." in rel.parts or rel.parts[:2] != ("curated", "switch"):
        raise ValueError("Invalid curated-file path")
    path = root / rel
    if not path.exists():
        return []
    document = json.loads(path.read_text(encoding="utf-8"))
    if document["schema_version"] != 1 or document["platform"] != "switch":
        raise ValueError(f"Invalid curated document: {rel}")
    if document["game"]["title"] != game["title"]:
        raise ValueError(f"Title mismatch in {rel}")
    edition = "build-" + game["build_id"].lower()
    editions = {item["id"]: item for item in document["editions"]}
    # Other archived Build IDs are real sources but do not cover this one.
    if edition not in editions:
        return []
    if editions[edition].get("build_id", "").upper() != game["build_id"]:
        raise ValueError(f"Contradictory Build ID in {rel}")
    candidates = []
    for cheat in document["cheats"]:
        for variant in cheat["variants"]:
            if variant["edition"] != edition:
                continue
            source = variant["source"]
            archive_rel = Path(source["archive_path"])
            if (archive_rel.is_absolute() or ".." in archive_rel.parts
                    or archive_rel.parts[0] != "archive"
                    or not (root / archive_rel).is_file()):
                raise ValueError(f"Missing or invalid archived source for {cheat['id']}")
            candidates.append({
                "id": cheat["id"],
                "name": cheat["name"],
                "source": source,
                "digest": fingerprint(variant["code"]),
            })
    return candidates


def reconcile(root: Path, report: dict) -> dict:
    if report.get("format") != "cheatarium-switch-exact-build-coverage-v1":
        raise ValueError("Unsupported coverage format")
    total_archived = total_linked = 0
    for game in report["games"]:
        candidates = archived_candidates(root, game)
        archived = linked = 0
        for row in game["sections"]:
            digest = row.get("source_code_sha256")
            # Smash's independent attributed source is still link-only and
            # not represented by source-body fingerprints in this public repo.
            matches = [item for item in candidates if digest and item["digest"] == digest]
            if matches:
                # Preserve a valid previously selected exact occurrence.
                pick = next((x for x in matches if
                    x["id"] == row["curated_cheat_id"] and
                    x["source"]["archive_path"] == row["archive_path"]), None)
                if pick is None:
                    heading = row["heading"][1:-1].strip()
                    if heading.startswith("♯"):
                        heading = heading[1:].strip()
                    pick = next((x for x in matches if x["name"].casefold() == heading.casefold()), matches[0])
                row["state"] = "archived_code_exact_match"
                row["curated_cheat_id"] = pick["id"]
                row["archive_path"] = pick["source"]["archive_path"]
                archived += 1
            else:
                row["state"] = "linked_only_no_public_code_body"
                row["curated_cheat_id"] = None
                row["archive_path"] = None
                linked += 1
        game["archived_exact_section_count"] = archived
        game["missing_body_count"] = linked
        if archived + linked != game["section_count"]:
            raise ValueError(f"Inconsistent game section count: {game['title']}")
        total_archived += archived
        total_linked += linked
    report["counts"] = {
        "games": len(report["games"]),
        "source_sections": total_archived + total_linked,
        "archived_exact_sections": total_archived,
        "linked_only_sections": total_linked,
    }
    return report


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true", help="Exit nonzero when the report is stale")
    action.add_argument("--write", action="store_true", help="Update archive coverage from curated files")
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    file = args.root / REPORT
    old = file.read_text(encoding="utf-8")
    report = reconcile(args.root, json.loads(old))
    payload = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.check:
        if payload != old:
            raise SystemExit("Switch source coverage is stale; run --write after curated source changes")
    else:
        file.write_text(payload, encoding="utf-8")
    print(json.dumps({"status": "current" if args.check else "updated", **report["counts"]}))


if __name__ == "__main__":
    main()
