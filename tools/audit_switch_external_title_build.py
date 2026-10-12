#!/usr/bin/env python3
"""Audit pinned Switch title/build labels across imported and link-only sources.

Native title/build pairs are exact filename comparisons, not trusted game
identity, matching cheat effects, authorship rights or gameplay verification.
Title-only JSON mirror labels remain separate from native build IDs.
"""
import argparse
import json
import re
from collections import Counter
from pathlib import Path

from external_bulk import ROOT, load as load_external, classify
from measure_switch_bulk_coverage import SHARKIVE, NX, source_pairs

REL = Path("generated/v1/reports/switch-external-title-build-coverage.json")
FORMAT = "cheatarium-switch-external-title-build-coverage-v1"
HEX = r"[0-9a-fA-F]{16}"
A = re.compile(rf"(?:Titles|NX-60FPS-RES-GFX-Cheats/titles)/({HEX})/cheats/({HEX})\.txt\Z")
I = re.compile(rf"atmosphere/titles/({HEX})/cheats/({HEX})\.txt\Z")
JSON_TITLE = re.compile(rf"(?:cheats|cheats_gbatemp)/({HEX})\.json\Z")
NATIVE = {"ademola-switch-native": A, "ibnux-switch-cheatdb": I}
TITLE_ONLY = {"hamlet-switch-cheatdb", "sthetix-switch-mirror"}
NON_NATIVE = {"rogamer97-asm-switch"}


def parsed_external(source_id, rows):
    native_pairs = Counter()
    title_only_counts = Counter()
    patch_count = 0
    other_count = 0
    for path, _, _ in rows:
        kind = classify(source_id, path)
        if kind == "patch":
            patch_count += 1
            continue
        if kind != "cheat":
            raise ValueError("External metadata must classify as cheat or patch")
        native_match = NATIVE[source_id].fullmatch(path) if source_id in NATIVE else None
        title_match = JSON_TITLE.fullmatch(path) if source_id in TITLE_ONLY else None
        if native_match:
            title, build = native_match.groups()
            native_pairs[(title.lower(), build.lower())] += 1
        elif title_match:
            title_only_counts[title_match.group(1).lower()] += 1
        else:
            other_count += 1
    if source_id in NATIVE and (other_count or title_only_counts):
        raise ValueError(f"Native Switch source path unexpectedly lacked exact title/build label: {source_id}")
    if source_id in TITLE_ONLY and (other_count or native_pairs):
        raise ValueError(f"JSON source path unexpectedly lacked an exact title label: {source_id}")
    return native_pairs, title_only_counts, patch_count, other_count


def compare(archived, externals):
    imported_stats = {}
    imported_titles = set()
    imported_pairs = set()
    per_import_pairs = {}
    for source in (SHARKIVE, NX):
        stats, titles, pairs = source_pairs(archived[source], source)
        imported_stats[source] = stats
        imported_titles.update(x for x in titles if re.fullmatch(HEX, x, flags=re.IGNORECASE))
        imported_pairs.update(pairs)
        per_import_pairs[source] = pairs
    source_rows = []
    external_pairs = set()
    native_sources = set()
    mirror_title_labels = set()
    link_source_ids = set()
    by_source = {}
    for index in externals:
        if index["platform"] != "switch":
            continue
        source = index["id"]
        if source not in set(NATIVE) | TITLE_ONLY | NON_NATIVE:
            raise ValueError(f"Unreviewed external Switch source format: {source}")
        if source in link_source_ids:
            raise ValueError(f"Duplicate pinned external source: {source}")
        link_source_ids.add(source)
        if index.get("rights_status") != "link_only_rights_review":
            raise ValueError(f"Source lacks expected rights block: {source}")
        pairs, titles, patches, ignored = parsed_external(source, index["files"])
        native_sources |= {source} if source in NATIVE else set()
        external_pairs.update(pairs)
        mirror_title_labels.update(titles)
        native_titles = {a for a, _ in pairs}
        source_rows.append({
            "id": source,
            "revision": index["snapshot_commit"],
            "link_only_file_occurrences": len(index["files"]),
            "native_title_build_file_occurrences": sum(pairs.values()),
            "distinct_native_title_labels": len(native_titles),
            "distinct_native_title_build_pairs": len(pairs),
            "native_pair_repeat_excess": sum(pairs.values()) - len(pairs),
            "native_pairs_also_in_imported_sources": len(set(pairs) & imported_pairs),
            "native_pairs_not_in_imported_sources": len(set(pairs) - imported_pairs),
            "native_unmatched_pairs_sample": [
                {"title_id_label": a, "build_id_label": b}
                for a, b in sorted(set(pairs) - imported_pairs)[:8]
            ],
            "title_only_json_file_occurrences": sum(titles.values()),
            "distinct_json_title_labels": len(titles),
            "json_titles_also_in_imported_sources": len(set(titles) & imported_titles),
            "json_titles_not_in_imported_sources": len(set(titles) - imported_titles),
            "patch_file_occurrences_not_assumed_cheats": patches,
            "other_cheat_documents_without_exact_native_ids": ignored,
        })
        by_source[source] = (pairs, titles)
    if link_source_ids != set(NATIVE) | TITLE_ONLY | NON_NATIVE:
        raise ValueError("Incomplete external Switch catalog review")
    if sum(x["link_only_file_occurrences"] for x in source_rows) != sum(
        len(x["files"]) for x in externals if x["platform"] == "switch"
    ):
        raise ValueError("Incomplete external Switch file accounting")
    all_titles = imported_titles | {a for a, _ in external_pairs} | mirror_title_labels
    native_external_titles = {a for a, _ in external_pairs}
    return {
        "schema_version": 1,
        "format": FORMAT,
        "comparison": "exact_source_filename_title_16hex_and_build_16hex_labels_only",
        "rights_unresolved_external_payloads_imported": False,
        "proves_distinct_games_or_new_cheats": False,
        "proves_code_equivalence_or_gameplay_compatibility": False,
        "json_mirror_has_build_id_evidence": False,
        "archived_sources": imported_stats,
        "external_sources": source_rows,
        "summary": {
            "imported_distinct_native_title_build_pairs": len(imported_pairs),
            "external_distinct_native_title_build_pairs": len(external_pairs),
            "native_pairs_shared_by_archive_and_external": len(imported_pairs & external_pairs),
            "native_pairs_only_in_external_sources": len(external_pairs - imported_pairs),
            "native_pairs_only_in_archived_sources": len(imported_pairs - external_pairs),
            "combined_distinct_native_title_build_pairs": len(imported_pairs | external_pairs),
            "imported_distinct_title_labels": len(imported_titles),
            "external_distinct_native_title_labels": len(native_external_titles),
            "mirror_distinct_title_only_json_labels": len(mirror_title_labels),
            "mirror_title_labels_without_archived_native_title": len(mirror_title_labels - imported_titles),
            "combined_distinct_title_labels_from_all_scope_types": len(all_titles),
        },
        "limitations": [
            "All identifiers are supplied by third-party filenames; no independent game-build confirmation.",
            "Exact title/build label equality does not imply a common executable cheat effect.",
            "Mirrored JSON title labels contain no comparable native 16-hex build ID in their filename.",
            "FPS, resolution and graphics patch file occurrences are deliberately not counted as native cheats.",
            "Rights-unresolved external cheat bodies remain link-only, regardless of apparent label novelty.",
        ],
    }


def build(root=ROOT):
    archived = {}
    for sid in (SHARKIVE, NX):
        doc = json.loads((root / "sources" / (sid + ".json")).read_text("utf-8"))
        if doc.get("id") != sid:
            raise ValueError("Wrong imported Switch source ID")
        archived[sid] = doc
    return compare(archived, load_external(root))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--write", action="store_true")
    modes.add_argument("--check", action="store_true")
    args = parser.parse_args()
    payload = json.dumps(build(), indent=2, ensure_ascii=False, sort_keys=True) + "\n"
    target = ROOT / REL
    if args.write:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(payload, "utf-8")
    elif not target.exists() or target.read_text("utf-8") != payload:
        raise SystemExit("Switch native/title-only external coverage report is stale or missing")
    print("OK: pinned Switch external pair and JSON title-only source coverage")


if __name__ == "__main__":
    main()
