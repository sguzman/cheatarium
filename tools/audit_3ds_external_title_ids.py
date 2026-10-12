#!/usr/bin/env python3
"""Compare native 3DS title-ID filename labels across imported and link-only sources.

Only source-declared 16-hex IDs are compared. No cheat bodies, game ROMs,
region equivalence, verified build compatibility or rights are inferred.
"""
import argparse
import json
import re
from collections import Counter
from pathlib import Path

from external_bulk import ROOT, load as load_external, classify

REL = Path("generated/v1/reports/3ds-external-title-id-coverage.json")
FORMAT = "cheatarium-3ds-native-title-id-coverage-v1"
SHARKIVE = re.compile(r"3ds/([0-9a-fA-F]{16})\.txt\Z")
ADEMOLA = re.compile(r"[^/]+/(000400[0-9a-fA-F]{10})\.txt\Z")
JOURNEY = re.compile(r"Cheats/[^/]+/([0-9a-fA-F]{16})\.txt\Z")
NATIVE = {"ademola-3ds-native": ADEMOLA, "journeyover-3ds-ctrpf": JOURNEY}
DOCUMENTS = {"rogamer97-asm-3ds"}


def tally_imported(rows):
    title_ids = Counter()
    other = []
    for row in rows:
        p = row["upstream_path"]
        if not p.startswith("3ds/"):
            continue
        match = SHARKIVE.fullmatch(p)
        if match:
            title_ids[match.group(1).lower()] += 1
        else:
            other.append(p)
    if other:
        raise ValueError("Sharkive 3DS filename absent exact 16-hex title ID")
    return title_ids


def classify_link_source(source, files):
    matcher = NATIVE.get(source)
    titles = Counter()
    unmatched = 0
    for path, _, _ in files:
        if classify(source, path) != "cheat":
            raise ValueError("3DS comparison saw an unsupported patch or metadata source")
        match = matcher.fullmatch(path) if matcher is not None else None
        if match:
            titles[match.group(1).lower()] += 1
        else:
            unmatched += 1
    if matcher is not None and unmatched:
        raise ValueError(f"3DS title-ID filename format drift for {source}")
    return titles, unmatched


def compare(sharkive, external_indexes):
    imported = tally_imported(sharkive["files"])
    external_union = set()
    rows = []
    seen = set()
    for external in external_indexes:
        if external["platform"] != "3ds":
            continue
        source = external["id"]
        if source in seen or source not in set(NATIVE) | DOCUMENTS:
            raise ValueError(f"Unexpected or duplicate 3DS source: {source}")
        if external.get("rights_status") != "link_only_rights_review":
            raise ValueError("Original 3DS code rights are not cleared")
        seen.add(source)
        titles, documents = classify_link_source(source, external["files"])
        external_union.update(titles)
        rows.append({
            "source_id": source,
            "revision": external["snapshot_commit"],
            "link_only_source_files": len(external["files"]),
            "native_title_id_file_occurrences": sum(titles.values()),
            "distinct_native_title_id_labels": len(titles),
            "native_title_id_repeat_file_occurrences": sum(titles.values()) - len(titles),
            "source_titles_also_in_archived_sharkive": len(set(titles) & set(imported)),
            "source_titles_missing_from_archived_sharkive": len(set(titles) - set(imported)),
            "source_only_title_id_samples": sorted(set(titles) - set(imported))[:12],
            "non_native_markdown_or_asm_documents": documents,
        })
    if seen != set(NATIVE) | DOCUMENTS:
        raise ValueError("3DS external title-ID catalog list incomplete")
    result = {
        "schema_version": 1,
        "format": FORMAT,
        "basis": "exact_16hex_title_id_filename_labels_only",
        "rights_blocked_external_codes_not_imported": True,
        "game_identity_verified": False,
        "region_build_effect_equivalence_verified": False,
        "source_metadata_only": True,
        "archived_sharkive_3ds_files": sum(imported.values()),
        "archived_sharkive_distinct_title_id_labels": len(imported),
        "external": rows,
        "summary": {
            "external_link_only_original_file_occurrences": sum(r["link_only_source_files"] for r in rows),
            "external_native_title_id_file_occurrences": sum(r["native_title_id_file_occurrences"] for r in rows),
            "external_distinct_title_id_labels": len(external_union),
            "title_ids_in_both_archive_and_external": len(external_union & set(imported)),
            "external_title_id_labels_absent_from_sharkive": len(external_union - set(imported)),
            "archived_title_id_labels_absent_from_external": len(set(imported) - external_union),
            "combined_distinct_3ds_title_id_labels": len(set(imported) | external_union),
            "non_native_3ds_code_documents": sum(r["non_native_markdown_or_asm_documents"] for r in rows),
        },
        "limitations": [
            "Title ID filename labels are supplied by different upstream authors, not confirmed ROM hashes.",
            "Two different file paths for the same ID are not proof of duplicate executable cheat effects.",
            "Many non-native source documents lack exact title-ID filenames and stay outside ID coverage.",
            "No rights-blocked cheat bodies are copied or independently verified.",
        ],
    }
    if result["summary"]["external_link_only_original_file_occurrences"] != sum(
            len(x["files"]) for x in external_indexes if x["platform"] == "3ds"):
        raise ValueError("3DS source file reconciliation failed")
    return result


def build(root=ROOT):
    original = json.loads((root / "sources" / "sharkive.json").read_text("utf-8"))
    if original.get("id") != "sharkive":
        raise ValueError("Wrong imported Sharkive inventory")
    return compare(original, load_external(root))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(build(), sort_keys=True, ensure_ascii=False, indent=2) + "\n"
    dest = ROOT / REL
    if args.write:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(rendered, "utf-8")
    elif not dest.exists() or dest.read_text("utf-8") != rendered:
        raise SystemExit("3DS native ID coverage report is stale or altered")
    print("OK: pinned 3DS title-ID filename-only coverage")


if __name__ == "__main__":
    main()
