#!/usr/bin/env python3
"""Compare rights-blocked upstream Git blobs against actually archived source blobs.

This audit uses pinned SHA-1 Git blob identities from source manifests, not
heuristic filenames, downloaded external cheat bodies or assumed game identities.
It does not change or imply redistribution rights for any external source.
"""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

from external_bulk import ROOT, load

REL = Path("generated/v1/reports/external-vs-archive-blobs.json")
FORMAT = "cheatarium-external-vs-archive-exact-blobs-v1"
SHA = re.compile(r"[0-9a-f]{40}\Z")


def load_archived(root=ROOT):
    result = []
    seen = set()
    for manifest in sorted((root / "sources").glob("*.json")):
        if manifest.name == "candidates.json":
            continue
        doc = json.loads(manifest.read_text("utf-8"))
        source_id = doc.get("id")
        rows = doc.get("files")
        archive_prefix = doc.get("archive_prefix", f"archive/{source_id}/")
        if (not isinstance(archive_prefix, str) or not archive_prefix.startswith("archive/")
                or not archive_prefix.endswith("/") or ".." in archive_prefix.split("/")):
            raise ValueError(f"Invalid source archive prefix: {manifest}")
        if (not isinstance(source_id, str) or not source_id
                or not isinstance(rows, list)):
            raise ValueError(f"Malformed archived manifest: {manifest}")
        for row in rows:
            if not isinstance(row, dict):
                raise ValueError("Archived source record is not an object")
            path = row.get("archive_path")
            sha = row.get("git_blob_sha")
            if (not isinstance(path, str)
                    or not path.startswith(archive_prefix)
                    or not isinstance(sha, str) or not SHA.fullmatch(sha)):
                raise ValueError(f"Invalid archived blob evidence: {source_id}, {path!r}")
            if path in seen:
                raise ValueError(f"Duplicate archived source path {path}")
            seen.add(path)
            result.append((sha, source_id, path))
    return result


def compile_report(archived_rows, external_indexes):
    archive_by_sha = defaultdict(list)
    for sha, source_id, path in archived_rows:
        if not SHA.fullmatch(sha):
            raise ValueError("Invalid archived Git blob SHA")
        archive_by_sha[sha].append((source_id, path))
    for rows in archive_by_sha.values():
        rows.sort()
    sources = []
    by_platform = defaultdict(lambda: {"reference_files": 0, "matches_archived_blob": 0,
                                        "unmatched_archive_blob": 0, "blob_hashes": set(),
                                        "matched_blob_hashes": set()})
    external_distinct = set()
    external_matched_distinct = set()
    shared_ref_count = 0
    total_external = 0
    sample = []
    for src in external_indexes:
        n = match = 0
        unique = set()
        matched_unique = set()
        local_samples = []
        for path, sha, size in src["files"]:
            if not SHA.fullmatch(sha) or not isinstance(size, int) or size < 0:
                raise ValueError("Invalid external Git source identity")
            n += 1
            unique.add(sha)
            external_distinct.add(sha)
            platform = by_platform[src["platform"]]
            platform["reference_files"] += 1
            platform["blob_hashes"].add(sha)
            if sha in archive_by_sha:
                match += 1
                shared_ref_count += 1
                matched_unique.add(sha)
                external_matched_distinct.add(sha)
                platform["matches_archived_blob"] += 1
                platform["matched_blob_hashes"].add(sha)
                if len(local_samples) < 4:
                    original_source, archived_path = archive_by_sha[sha][0]
                    local_samples.append({
                        "external_path": path,
                        "git_blob_sha": sha,
                        "archived_source_id": original_source,
                        "archived_path": archived_path,
                    })
            else:
                platform["unmatched_archive_blob"] += 1
        sources.append({
            "source_id": src["id"],
            "platform": src["platform"],
            "reference_files": n,
            "distinct_external_git_blobs": len(unique),
            "matches_archived_blob": match,
            "distinct_git_blobs_already_archived": len(matched_unique),
            "references_without_archived_byte_match": n - match,
            "distinct_git_blobs_without_archived_byte_match": len(unique - matched_unique),
            "exact_byte_match_examples": local_samples,
        })
    platforms = []
    for platform, values in sorted(by_platform.items()):
        platforms.append({
            "platform": platform,
            "reference_files": values["reference_files"],
            "matches_archived_blob": values["matches_archived_blob"],
            "references_without_archived_byte_match": values["unmatched_archive_blob"],
            "distinct_external_git_blobs": len(values["blob_hashes"]),
            "distinct_git_blobs_already_archived": len(values["matched_blob_hashes"]),
            "distinct_git_blobs_without_archived_byte_match": len(
                values["blob_hashes"] - values["matched_blob_hashes"]
            ),
        })
    total_external = sum(row["reference_files"] for row in sources)
    if (total_external != sum(row["reference_files"] for row in platforms)
            or shared_ref_count != sum(row["matches_archived_blob"] for row in sources)
            or shared_ref_count != sum(row["matches_archived_blob"] for row in platforms)
            or not (0 <= len(external_matched_distinct) <= len(external_distinct))):
        raise ValueError("External/archive exact-byte reconciliation failed")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "git-blob-sha1-exact-original-bytes-only",
        "no_cheat_payloads_in_report": True,
        "proves_game_or_cheat_equivalence": False,
        "establishes_external_redistribution_rights": False,
        "proves_gameplay_compatibility": False,
        "archive_manifest_file_occurrences": len(archived_rows),
        "archive_distinct_git_blobs": len(archive_by_sha),
        "external_reference_files": total_external,
        "external_distinct_git_blobs": len(external_distinct),
        "external_references_matching_archived_bytes": shared_ref_count,
        "external_references_without_archived_byte_match": total_external - shared_ref_count,
        "external_distinct_git_blobs_matching_archive": len(external_matched_distinct),
        "external_distinct_git_blobs_without_archive_byte_match":
            len(external_distinct - external_matched_distinct),
        "sources": sources,
        "platforms": platforms,
    }


def build(root=ROOT):
    return compile_report(load_archived(root), load(root))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_mutually_exclusive_group(required=True)
    commands.add_argument("--write", action="store_true")
    commands.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(build(), indent=2, ensure_ascii=False) + "\n"
    dest = ROOT / REL
    if args.write:
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(rendered, encoding="utf-8")
    elif not dest.is_file() or dest.read_text("utf-8") != rendered:
        raise SystemExit("External vs archive byte-identity audit is stale or modified")
    doc = json.loads(rendered)
    print(f"OK: {doc['external_reference_files']} external files compared to "
          f"{doc['archive_manifest_file_occurrences']} archived originals; "
          f"{doc['external_references_matching_archived_bytes']} exact-byte matches")


if __name__ == "__main__":
    main()
