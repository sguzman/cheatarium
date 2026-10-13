#!/usr/bin/env python3
"""Count source-native Wii Gecko cheat groups/hex code pairs in a verified local cache.

Never exports or logs any original cheat text, code bytes, author labels,
game names, per-file code fingerprints or ROM data. Exact original Git blobs
are verified offline against pinned upstream metadata first. Results are
source-family aggregates, NOT tested effects or distinct game counts.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import re

from hydrate_wii_bulk import check, default_destination

PAIR = re.compile(r"^[0-9A-Fa-f]{8}\s+[0-9A-Fa-f]{8}$")
SECTION = re.compile(r"^\[([A-Za-z_]+)\]$")
FORMAT = "cheatarium-wii-original-gecko-group-syntax-audit-v1"


def inspect_text(raw, name):
    text = raw.decode("utf-8-sig")
    ini = name.lower().endswith(".ini")
    if not (ini or name.lower().endswith(".txt")):
        raise ValueError("Unexpected pinned native Wii code file suffix")
    active = not ini
    groups = 0
    group_has_pairs = False
    group_with_pairs = 0
    code_pair_lines = 0
    enabled_marker_names = 0
    comments = 0
    unparsed = 0
    declared_gecko_sections = 0
    for line in text.splitlines():
        line = line.strip()
        if not line:
            continue
        match = SECTION.fullmatch(line) if ini else None
        if match:
            active = match.group(1) == "Gecko"
            if active:
                declared_gecko_sections += 1
            continue
        if ini and not active:
            if line.startswith("$") and line != "$":
                enabled_marker_names += 1
            continue
        if line.startswith("$"):
            if group_has_pairs:
                group_with_pairs += 1
            groups += 1
            group_has_pairs = False
            continue
        if line.startswith("*"):
            comments += 1
            continue
        if PAIR.fullmatch(line):
            code_pair_lines += 1
            if groups:
                group_has_pairs = True
            continue
        unparsed += 1
    if group_has_pairs:
        group_with_pairs += 1
    return {
        "declared_cheat_header_occurrences": groups,
        "cheat_headers_with_hex_code_pairs": group_with_pairs,
        "eight_hex_word_pair_line_occurrences": code_pair_lines,
        "source_comment_line_occurrences": comments,
        "unrecognized_active_section_lines": unparsed,
        "declared_gecko_ini_sections": declared_gecko_sections,
        "source_enabled_selector_name_occurrences": enabled_marker_names,
    }


def summarize(cache=default_destination()):
    cache = Path(cache).expanduser().resolve()
    manifest = check(cache)
    by_source = {}
    for item in manifest["records"]:
        sid = item["source_id"]
        path = item["path"]
        counts = inspect_text((cache / path).read_bytes(), path)
        summary = by_source.setdefault(sid, Counter())
        summary["original_source_file_occurrences"] += 1
        summary.update(counts)
    if sum(x["original_source_file_occurrences"] for x in by_source.values()) != 29:
        raise ValueError("Pinned Wii original file count changed")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "pinned-local-verified-native-gecko-syntax-groups",
        "private_source_code_bytes_exported": False,
        "individual_code_line_values_or_hashes_exported": False,
        "gameplay_verified": False,
        "game_and_revision_identity_verified": False,
        "rights_to_republish_original_codes_established": False,
        "source_file_occurrences": 29,
        "source_families": len(by_source),
        "source_family_aggregates": {
            key: dict(sorted(value.items())) for key, value in sorted(by_source.items())
        },
        "limitations": [
            "A $-prefixed native Gecko header is a source-defined cheat group, not a tested gameplay effect.",
            "Eight-hex-word lines are native syntax elements, not standalone selectable cheat effects.",
            "INI enabled selectors are source file defaults and not game compatibility evidence.",
            "Source GameCube/controller patches can comprise many instruction lines per single cheat group.",
        ],
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--cache", type=Path, default=default_destination())
    p.add_argument("--output", type=Path)
    args = p.parse_args()
    report = summarize(args.cache)
    encoded = json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True) + "\n"
    if args.output:
        args.output.write_text(encoded, "utf-8")
    print("BEGIN_NINTENDO_WII_GECKO_SYNTAX_AUDIT")
    print(encoded)
    print("END_NINTENDO_WII_GECKO_SYNTAX_AUDIT")


if __name__ == "__main__":
    main()
