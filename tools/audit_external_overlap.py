#!/usr/bin/env python3
"""Audit exact Git-blob overlap across rights-unresolved external source catalogs.

No upstream payloads are downloaded or read. SHA-1 Git blob IDs describe exact
byte identity, not distinct games, verified cheat effects, licensing or
compatibility. All pinned inputs are metadata-only pinned source references.
"""
import argparse
import json
from collections import defaultdict
from pathlib import Path

from external_bulk import ROOT, load

REL = Path("generated/v1/reports/external-overlap.json")
FORMAT = "cheatarium-external-blob-overlap-v1"


def build(root=ROOT):
    sources = load(root)
    occurrences = defaultdict(list)
    per_platform = defaultdict(list)
    for source in sources:
        identifier = source["id"]
        platform = source["platform"]
        for path, blob, size in source["files"]:
            item = {"source_id": identifier, "platform": platform,
                    "path": path, "git_blob_sha": blob, "bytes": size}
            occurrences[blob].append(item)
            per_platform[platform].append(item)

    source_rows = []
    for source in sources:
        hashes = [blob for _, blob, _ in source["files"]]
        distinct = set(hashes)
        cross_source = sum(
            len({record["source_id"] for record in occurrences[blob]}) > 1
            for blob in hashes
        )
        source_rows.append({
            "id": source["id"], "platform": source["platform"],
            "reference_files": len(hashes),
            "distinct_git_blobs": len(distinct),
            "repeated_blob_excess_occurrences_within_source": len(hashes) - len(distinct),
            "references_with_blob_shared_by_another_source": cross_source,
            "source_declared_file_types": source["file_counts"],
        })

    platform_rows = []
    for platform, rows in sorted(per_platform.items()):
        unique = {r["git_blob_sha"] for r in rows}
        platform_rows.append({
            "platform": platform, "reference_files": len(rows),
            "distinct_git_blobs": len(unique),
            "repeated_blob_excess_occurrences": len(rows) - len(unique),
            "source_catalogs": sorted({r["source_id"] for r in rows}),
        })

    shared = []
    for blob, records in sorted(occurrences.items()):
        different_sources = {record["source_id"] for record in records}
        if len(different_sources) > 1:
            shared.append({
                "git_blob_sha": blob,
                "source_count": len(different_sources),
                "references": sorted(records, key=lambda r: (r["source_id"], r["path"])),
            })
    n = sum(row["reference_files"] for row in source_rows)
    distinct_global = len(occurrences)
    repeated_occurrences = sum(len(rows) for rows in occurrences.values() if len(rows) > 1)
    cross_source_occurrences = sum(len(row["references"]) for row in shared)
    if n != 43409 or len(sources) != 48:
        raise ValueError("Pinned external source inventory count changed")
    if (n - distinct_global != sum(
            row["repeated_blob_excess_occurrences_within_source"] for row in source_rows
            ) + sum(row["source_count"] - 1 for row in shared)):
        raise ValueError("Source overlap accounting invariant failed")
    # Each Git blob is a byte-identity group. Different paths can refer to
    # the same bytes, but *not* a proven identical game, cheat or executable.
    return {
        "schema_version": 1, "format": FORMAT,
        "metadata_only": True, "rights_status": "link_only_rights_review",
        "method": "exact-original-git-blob-sha-only",
        "independently_verified_games_or_effects": False,
        "archive_payloads_included": False,
        "reference_files": n,
        "source_catalogs": len(source_rows),
        "external_platforms": len(platform_rows),
        "distinct_git_blobs": distinct_global,
        "repeated_blob_excess_occurrences": n - distinct_global,
        "references_in_repeated_blob_groups": repeated_occurrences,
        "cross_source_shared_blob_groups": len(shared),
        "cross_source_shared_references": cross_source_occurrences,
        "sources": source_rows,
        "platforms": platform_rows,
        "cross_source_exact_blob_matches": shared,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    dest = ROOT / REL
    if args.write:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(data, encoding="utf-8")
    elif not dest.is_file() or dest.read_text("utf-8") != data:
        raise SystemExit("External exact-blob overlap audit is stale or altered")
    print(f"OK: {data.count('source_declared_file_types')} external source overlap rows")


if __name__ == "__main__":
    main()
