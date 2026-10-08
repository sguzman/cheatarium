#!/usr/bin/env python3
"""Discover near-identical unresolved SNES '+' source text, without guessing semantics.

This offline research aid groups original source *occurrences* that share all
but one plus-separated component. Patterns never establish code equivalence,
simultaneous execution, correct game identity, or ROM compatibility.
"""
import argparse
import gzip
import json
from collections import defaultdict
from pathlib import Path

from inspect_snes_joins import dossier

ROOT = Path(__file__).resolve().parents[1]
PAGE_SIZE = 500
SAMPLE_SIZE = 3


def analyze(bundle, game_key, *, publications=None, source_record_id=None,
            min_members=2, offset=0, limit=25):
    if min_members < 2 or offset < 0 or limit < 1 or limit > 100:
        raise ValueError("Invalid minimum family size, page offset or page limit")
    first = dossier(bundle, game_key, offset=0, limit=PAGE_SIZE,
                    source_record_id=source_record_id,
                    publication_registry=publications)
    entries = list(first["occurrences"])
    while len(entries) < first["total_unresolved_source_occurrences"]:
        page = dossier(bundle, game_key, offset=len(entries), limit=PAGE_SIZE,
                       source_record_id=source_record_id,
                       publication_registry=publications)
        if not page["occurrences"]:
            raise ValueError("Incomplete unresolved source dossier pagination")
        entries.extend(page["occurrences"])

    candidate_groups = defaultdict(list)
    for entry in entries:
        # Trim formatting only for TEXT pattern comparison; preserve source
        # text unchanged for all returned records.
        components = tuple(part.strip() for part in entry["original_code"].split("+"))
        if len(components) < 2 or any(not component for component in components):
            continue
        for variable_position in range(len(components)):
            signature = tuple(
                None if i == variable_position else part
                for i, part in enumerate(components)
            )
            key = (entry["source_record_id"], variable_position, signature)
            candidate_groups[key].append((entry, components[variable_position]))

    families = []
    for (record_id, position, signature), members in candidate_groups.items():
        variants = {part for _, part in members}
        if len(members) < min_members or len(variants) < 2:
            continue
        members.sort(key=lambda x: (x[0]["source_ordinal"], x[0]["original_code"]))
        sample = [{
            "source_ordinal": entry["source_ordinal"],
            "original_code": entry["original_code"],
            "original_description": entry["original_description"],
            "varying_component": component,
        } for entry, component in members[:SAMPLE_SIZE]]
        families.append({
            "source_record_id": record_id,
            "source_git_blob_sha": members[0][0]["provenance"]["git_blob_sha"],
            "segment_count": len(signature),
            "variable_component_position_zero_based": position,
            "fixed_text_components": list(signature),
            "original_occurrences": len(members),
            "distinct_varying_text_components": len(variants),
            "publication_witnessed_occurrences": sum(
                "historical_publication_witness" in entry for entry, _ in members
            ),
            "sample_originals": sample,
        })
    families.sort(key=lambda f: (
        -f["original_occurrences"],
        f["source_record_id"],
        f["variable_component_position_zero_based"],
        tuple(part or "" for part in f["fixed_text_components"]),
    ))
    selected = families[offset:offset + limit]
    return {
        "schema_version": 1,
        "platform": "snes",
        "scope": "one-variable-source-code-text-pattern-candidates",
        "candidate_game_key": game_key,
        "source_record_id_filter": source_record_id,
        "identity_confidence": "filename-derived-candidates-only",
        "comparison_basis": "literal-plus-separated-components-trimmed-for-text-comparison",
        "relation": "unresolved",
        "families_may_overlap": True,
        "pattern_is_not_code_equivalence": True,
        "execution_or_rom_compatibility_verified": False,
        "safe_to_combine_or_auto_apply": False,
        "unresolved_source_occurrences_examined": len(entries),
        "historical_publication_witnesses": first["historical_publication_witnesses"],
        "min_family_occurrences": min_members,
        "total_pattern_families": len(families),
        "offset": offset,
        "limit": limit,
        "returned": len(selected),
        "has_more": offset + len(selected) < len(families),
        "families": selected,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--game-key", required=True)
    parser.add_argument("--source-record-id")
    parser.add_argument("--min-members", type=int, default=2)
    parser.add_argument("--offset", type=int, default=0)
    parser.add_argument("--limit", type=int, default=25)
    args = parser.parse_args()
    with gzip.open(args.root / "generated/v1/snes.json.gz",
                   "rt", encoding="utf-8") as stream:
        bundle = json.load(stream)
    registry = json.loads(
        (args.root / "interpretations/v1/snes-published-groups.json")
        .read_text(encoding="utf-8")
    )
    result = analyze(
        bundle, args.game_key, publications=registry,
        source_record_id=args.source_record_id,
        min_members=args.min_members, offset=args.offset, limit=args.limit
    )
    if not result["unresolved_source_occurrences_examined"]:
        parser.error("No unresolved entries match the candidate/source filters")
    print(json.dumps(result, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
