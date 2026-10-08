#!/usr/bin/env python3
"""Inspect unresolved SNES '+' source entries without assigning code semantics.

This is an offline research aid, not a cheat executor or a ROM compatibility
resolver. It reads a generated Cheatarium bundle and prints a deterministic
JSON dossier retaining original descriptions, code text and provenance.
"""
import argparse
import gzip
import json
from collections import Counter
from pathlib import Path

from validate_snes_publications import validate_registry

ROOT = Path(__file__).resolve().parents[1]
MAX_PAGE_SIZE = 500


def dossier(bundle, game_key, *, offset=0, limit=50, source_record_id=None,
            publication_registry=None):
    if bundle.get("schema_version") != 1 or bundle.get("platform") != "snes":
        raise ValueError("Expected a version-one SNES source bundle")
    if not isinstance(game_key, str) or not game_key:
        raise ValueError("A nonempty candidate game key is required")
    if offset < 0 or not 1 <= limit <= MAX_PAGE_SIZE:
        raise ValueError("Offset must be nonnegative and page size between 1 and 500")

    reports = {}
    if publication_registry is not None:
        validate_registry(publication_registry, bundle)
        reports = {(r["source_record_id"], r["source_ordinal"]): r
                   for r in publication_registry["records"]}
    matches = []
    for record in bundle["records"]:
        record_id = record["id"]
        candidate = record["candidate_game_key"] or f"unresolved:{record_id}"
        if candidate != game_key or (
            source_record_id is not None and record_id != source_record_id
        ):
            continue
        for cheat in record["codes"]:
            raw = cheat.get("code")
            if cheat.get("role") != "code" or not isinstance(raw, str) or "+" not in raw:
                continue
            composition = cheat.get("composition")
            if not isinstance(composition, dict) or composition.get("relation") not in (
                "unresolved", "revision-alternatives"
            ):
                raise ValueError(f"Unclassified source join: {record_id} ordinal {cheat['ordinal']}")
            if composition["relation"] == "revision-alternatives":
                continue
            if (composition.get("evidence") or composition.get("alternatives")
                    or composition.get("rom_match_verified")
                    or composition.get("simultaneous_execution_confirmed")):
                raise ValueError(f"Unsupported evidence on unresolved join: {record_id}")
            occurrence = {
                "source_record_id": record_id,
                "source_ordinal": cheat["ordinal"],
                "original_description": cheat.get("description"),
                "original_code": raw,
                "original_source_enabled": cheat.get("source_enabled"),
                "original_verification": cheat.get("verification"),
                "text_segment_count": len(raw.split("+")),
                "title_hint": record["title_hint"],
                "region_hint": record.get("region_hint"),
                "declared_device_hint": record.get("format_hint"),
                "provenance": record["provenance"],
            }
            report = reports.get((record_id, cheat["ordinal"]))
            if report is not None:
                occurrence["historical_publication_witness"] = {
                    "published_effect_description": report["published_effect_description"],
                    "publication": report["publication"],
                    "execution_observed": False,
                    "rom_match_verified": False,
                    "safe_to_auto_apply": False,
                }
            matches.append(occurrence)
    matches.sort(key=lambda x: (x["source_record_id"], x["source_ordinal"]))
    record_counts = Counter(x["source_record_id"] for x in matches)
    segment_counts = Counter(x["text_segment_count"] for x in matches)
    page = matches[offset:offset + limit]
    return {
        "schema_version": 1,
        "platform": "snes",
        "scope": "unresolved-source-code-plus-semantics",
        "candidate_game_key": game_key,
        "identity_confidence": "filename-derived-candidates-only",
        "relation": "unresolved",
        "effect_or_compatibility_verified": False,
        "source_enabled_is_not_activation": True,
        "total_unresolved_source_occurrences": len(matches),
        "distinct_original_code_strings": len({x["original_code"] for x in matches}),
        "distinct_source_records": len(record_counts),
        "historical_publication_witnesses": sum(
            "historical_publication_witness" in x for x in matches),
        "source_record_counts": [
            {"source_record_id": key, "count": count}
            for key, count in sorted(record_counts.items(), key=lambda x: (-x[1], x[0]))
        ],
        "text_segment_count_distribution": [
            {"text_segment_count": n, "count": count}
            for n, count in sorted(segment_counts.items())
        ],
        "offset": offset,
        "limit": limit,
        "returned": len(page),
        "has_more": offset + len(page) < len(matches),
        "occurrences": page,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--game-key", required=True,
                        help="Exact advisory game key in the composition priority queue")
    parser.add_argument("--source-record-id", help="Optional exact original source record ID")
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--limit", type=int, default=50)
    args = parser.parse_args()
    with gzip.open(args.root / "generated/v1/snes.json.gz", "rt", encoding="utf-8") as stream:
        bundle = json.load(stream)
    registry = json.loads((args.root / "interpretations/v1/snes-published-groups.json")
                          .read_text(encoding="utf-8"))
    result = dossier(bundle, args.game_key, offset=args.offset, limit=args.limit,
                     source_record_id=args.source_record_id,
                     publication_registry=registry)
    if not result["total_unresolved_source_occurrences"]:
        parser.error("No unresolved source joins match the selected candidate and source")
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
