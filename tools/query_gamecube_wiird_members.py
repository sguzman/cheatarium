#!/usr/bin/env python3
"""Offline read-only lookup of original GameCube WiiRD ZIP member filename hints.

Reads only provenance metadata from a pinned upstream regional ZIP inventory.
No cheat body is present and a source filename does not confirm game identity.
"""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / "research/nintendo-gamecube-wiird-member-claims-v1.json"
FORMAT = "cheatarium-nintendo-gamecube-wiird-member-references-v1"
REGIONS = {"NTSC-J": 8, "NTSC-U": 138, "PAL": 229}


def query(document, needle="", region=None, offset=0, limit=20):
    if (document.get("format") != FORMAT
            or document.get("platform") != "gamecube"
            or document.get("read_only_filename_metadata") is not True
            or document.get("rights_status") != "link_only_rights_review"
            or document.get("cheat_bodies_included") is not False
            or document.get("game_identifiers_or_revisions_confirmed") is not False
            or document.get("gameplay_tested") is not False
            or document.get("reference_count") != 375
            or document.get("file_occurrences_by_region") != REGIONS):
        raise ValueError("Invalid source-only GameCube member claims")
    records = document.get("records")
    if not isinstance(records, list) or len(records) != 375:
        raise ValueError("GameCube regional original inventory incomplete")
    if region and region not in REGIONS:
        raise ValueError("Unsupported region")
    if limit < 0 or limit > 100 or offset < 0:
        raise ValueError("Invalid pagination")
    q = needle.casefold()
    hits = [
        record for record in records
        if (not region or record["region"] == region)
        and (q in record["native_zip_member_path"].casefold()
             or q in record["source_filename_title_hint"].casefold())
    ]
    return {
        "schema": "cheatarium.gamecube_zip_member_lookup.v1",
        "query": needle,
        "region": region,
        "offset": offset,
        "limit": limit,
        "total": len(hits),
        "returned": len(hits[offset:offset + limit]),
        "has_more": offset + limit < len(hits),
        "original_zip_member_paths_only": True,
        "code_bodies_included": False,
        "game_identity_or_cheat_effects_verified": False,
        "individual_author_redistribution_rights_cleared": False,
        "records": hits[offset:offset + limit],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--query", default="")
    parser.add_argument("--region", choices=sorted(REGIONS))
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--limit", type=int, default=20)
    parser.add_argument("--source", type=Path, default=DEFAULT)
    args = parser.parse_args()
    doc = json.loads(args.source.read_text("utf-8"))
    print(json.dumps(query(doc, args.query, args.region, args.offset, args.limit),
                     indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
