#!/usr/bin/env python3
"""Read-only local Wii Gecko search with explicit game/revision/code selection.

All 29 cached native files are checked against original pinned Git blobs
before search. Metadata is the default. Actual code text is shown only with
--show-code, an exact six-character game ID, exactly one selected native
file, and a one-based --group-index. The tool never executes or enables a code.
"""
import argparse
import json
from pathlib import Path
import re
import sys
import urllib.parse

from hydrate_wii_bulk import (
    SOURCE_IDS, check, default_destination, pinned_sources,
)

GAME_STEM = re.compile(r"^([A-Z0-9]{6})(?:v(\d+)|_(mp[123]))?$")
CODE_PAIR = re.compile(r"^[0-9A-Fa-f]{8}[ \t]+[0-9A-Fa-f]{8}$")
SECTION = re.compile(r"^\[([A-Za-z_]+)\]$")
SCHEMA = "cheatarium.private_wii_gecko_lookup.v1"
MAX_RESULTS = 100
GAME_FAMILY_HINTS = {
    "wii-accf-gecko-native": "Animal Crossing: City Folk / Deluxe",
    "wii-metroid-prime-gecko-native": "Metroid Prime / Trilogy",
    "wii-metroid-other-m-gecko-native": "Metroid: Other M",
    "wii-taiko-gecko-native": "Taiko no Tatsujin",
}
SOURCE_CAVEATS = {
    "wii-accf-gecko-native": "City Folk source revision tags v0/v1/v2 are incompatible patch-site variants; match your own disc revision externally before use.",
    "wii-metroid-prime-gecko-native": "Trilogy source variants base/mp1/mp2/mp3 are not interchangeable; game and executable build have not been authenticated.",
    "wii-metroid-other-m-gecko-native": "Original upstream runtime testing was limited; region-labeled sources do not prove revision compatibility.",
    "wii-taiko-gecko-native": "Different Taiko Wii disc IDs are different native game variants; no emulator or hardware compatibility test was performed.",
}


def source_identity(relative):
    """Filename tokens are unverified SOURCE claims, not authenticated ROM IDs."""
    stem = Path(relative).stem
    match = GAME_STEM.fullmatch(stem)
    if not match:
        raise ValueError("Unexpected pinned Wii Gecko source-native filename: " + relative)
    disc, revision, variant = match.groups()
    return {
        "claimed_disc_id": disc,
        "source_revision_token": "v" + revision if revision is not None else None,
        "source_variant_token": variant or "base",
        "native_file_stem": stem,
    }


def parse_gecko(raw, path):
    """Read source-native groups; distinguish selectors from hex instruction pairs."""
    if not path.endswith((".txt", ".ini")):
        raise ValueError("Unsupported Gecko source type")
    ini = path.endswith(".ini")
    active = not ini
    found_gecko = False
    enabled = set()
    groups = []
    current = None
    for original in raw.decode("utf-8-sig").splitlines():
        line = original.strip()
        if not line:
            continue
        sec = SECTION.fullmatch(line) if ini else None
        if sec:
            active = sec.group(1) == "Gecko"
            found_gecko = found_gecko or active
            current = None
            continue
        if ini and not active:
            if line.startswith("$") and line != "$":
                enabled.add(line[1:].strip())
            continue
        if line.startswith("$") and len(line) > 1:
            current = {"group_index": len(groups) + 1, "title": line[1:].strip(),
                       "notes": [], "code_lines": []}
            groups.append(current)
        elif line.startswith("*"):
            if current:
                current["notes"].append(line[1:].strip())
        elif CODE_PAIR.fullmatch(line):
            if current is None:
                raise ValueError("Orphan code instruction outside native cheat group")
            current["code_lines"].append(line)
        else:
            raise ValueError("Unrecognized source-native Gecko code line")
    if ini and not found_gecko:
        raise ValueError("Missing native [Gecko] section")
    if not groups:
        raise ValueError("Source-native file has no selectable Gecko groups")
    for g in groups:
        g["source_enabled"] = g["title"] in enabled if ini else None
        g["instruction_pair_count"] = len(g["code_lines"])
        if not g["code_lines"]:
            raise ValueError("Empty Gecko header is not a selectable code group")
    return groups


def verified_index(cache, root=None):
    """Retain revision selection and source credit; do not duplicate code in index."""
    root = root or Path(__file__).resolve().parents[1]
    cache = Path(cache).expanduser().resolve()
    manifest = check(cache, root)
    sources = {s["id"]: s for s in pinned_sources(root)}
    entries = []
    for item in manifest["records"]:
        identity = source_identity(item["original_path"])
        source = sources[item["source_id"]]
        raw = (cache / item["path"]).read_bytes()
        groups = parse_gecko(raw, item["original_path"])
        upstream_url = (source["repository"] + "/blob/" + source["snapshot_commit"] +
                        "/" + urllib.parse.quote(item["original_path"], safe="/"))
        entries.append({
            **identity,
            "source_id": item["source_id"],
            "game_family_title_hint": GAME_FAMILY_HINTS[item["source_id"]],
            "source_compatibility_warning": SOURCE_CAVEATS[item["source_id"]],
            "local_path": item["path"],
            "upstream_original_url": upstream_url,
            "original_git_blob_sha1": item["source_git_blob_sha1"],
            "source_rights": "link_only_rights_review",
            "groups": groups,
        })
    if len(entries) != 29 or sum(len(e["groups"]) for e in entries) != 45:
        raise ValueError("Original Wii Gecko source group inventory drift")
    entries.sort(key=lambda e: (e["claimed_disc_id"], e["source_revision_token"] or "",
                                e["source_variant_token"], e["source_id"]))
    return entries


def query(entries, game_id=None, revision=None, variant=None, title=None,
          source_id=None, limit=50, offset=0, show_code=False, group_index=None,
          game_title=None):
    if game_id is not None:
        game_id = game_id.upper()
        if not re.fullmatch("[A-Z0-9]{6}", game_id):
            raise ValueError("Expected exact six-character source disc ID")
    if revision is not None:
        revision = revision.lower().removeprefix("v")
        if not revision.isdecimal():
            raise ValueError("Use source revision number, e.g. --revision 0")
        revision = "v" + revision
    if variant is not None and variant not in ("base", "mp1", "mp2", "mp3"):
        raise ValueError("Unknown source variant token")
    if source_id is not None and source_id not in SOURCE_IDS:
        raise ValueError("Unknown original source family")
    if not (0 <= offset and 1 <= limit <= MAX_RESULTS):
        raise ValueError("Invalid result pagination")
    if group_index is not None and group_index < 1:
        raise ValueError("--group-index must be 1-based")
    if show_code and (not game_id or group_index is None or offset):
        raise ValueError("--show-code requires --game-id, --group-index and offset 0")
    if group_index is not None and not show_code:
        raise ValueError("--group-index requires explicit --show-code")

    q = (title or "").casefold()
    game_q = (game_title or "").casefold()
    matches = []
    file_candidates = 0
    for entry in entries:
        if game_id and entry["claimed_disc_id"] != game_id:
            continue
        if revision and entry["source_revision_token"] != revision:
            continue
        if variant and entry["source_variant_token"] != variant:
            continue
        if source_id and entry["source_id"] != source_id:
            continue
        if game_q and game_q not in entry["game_family_title_hint"].casefold():
            continue
        file_candidates += 1
        groups = [g for g in entry["groups"]
                  if not q or q in g["title"].casefold() or
                  any(q in note.casefold() for note in g["notes"])]
        if q and not groups:
            continue
        matches.append((entry, groups))
    if show_code and (file_candidates != 1 or len(matches) != 1):
        raise ValueError("Ambiguous or absent native Wii file: select exact --revision / --variant; never silently choose a build")
    records = []
    selected = matches if show_code else matches[offset:offset + limit]
    for entry, groups in selected:
        if show_code:
            groups = [g for g in groups if g["group_index"] == group_index]
            if len(groups) != 1:
                raise ValueError("Unknown or filtered native Gecko group index")
        records.append({
            "claimed_disc_id": entry["claimed_disc_id"],
            "source_revision_token": entry["source_revision_token"],
            "source_variant_token": entry["source_variant_token"],
            "native_file_stem": entry["native_file_stem"],
            "game_family_title_hint": entry["game_family_title_hint"],
            "source_compatibility_warning": entry["source_compatibility_warning"],
            "original_source_id": entry["source_id"],
            "original_source_path": entry["local_path"],
            "original_git_blob_sha1": entry["original_git_blob_sha1"],
            "upstream_original_url": entry["upstream_original_url"],
            "rights_status": entry["source_rights"],
            "cheat_groups": [{
                "index": group["group_index"],
                "title": group["title"],
                "source_notes": group["notes"],
                "native_instruction_pairs": group["instruction_pair_count"],
                "source_enabled": group["source_enabled"],
                **({"code_lines": group["code_lines"]} if show_code else {}),
            } for group in groups],
        })
    return {
        "schema": SCHEMA,
        "query": {"game_id": game_id, "revision": revision, "variant": variant,
                  "title": title, "game_title": game_title, "source_id": source_id},
        "file_matches": len(matches),
        "ambiguous_source_variants": len(matches) > 1,
        "returned": len(records),
        "offset": offset,
        "limit": limit,
        "has_more": offset + limit < len(matches) if not show_code else False,
        "original_code_text_included": show_code,
        "private_cache_verified_against_original_git": True,
        "code_executed_or_enabled": False,
        "game_build_authenticated": False,
        "gameplay_effect_verified": False,
        "permission_to_republish_original_code": False,
        "records": records,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=Path, default=default_destination())
    parser.add_argument("--game-id", help="Exact source-declared 6-character disc ID")
    parser.add_argument("--revision", help="Source filename revision number, e.g. 0 (no inference)")
    parser.add_argument("--variant", choices=["base", "mp1", "mp2", "mp3"])
    parser.add_argument("--source-id", choices=SOURCE_IDS)
    parser.add_argument("--title", help="Case-insensitive code-group title/notes search")
    parser.add_argument("--game-title", help="Search one of the four original Wii source game-family labels")
    parser.add_argument("--limit", type=int, default=50)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--group-index", type=int, help="One-based group in original file")
    parser.add_argument("--show-code", action="store_true",
                        help="Explicitly display original native group code text in your own terminal")
    args = parser.parse_args()
    try:
        output = query(verified_index(args.cache), args.game_id, args.revision,
                       args.variant, args.title, args.source_id, args.limit,
                       args.offset, args.show_code, args.group_index, args.game_title)
    except (ValueError, UnicodeError, OSError) as exc:
        parser.error(str(exc))
    print(json.dumps(output, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
