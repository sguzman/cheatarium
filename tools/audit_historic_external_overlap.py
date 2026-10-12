#!/usr/bin/env python3
"""Measure exact Git-blob overlap of historical and other link-only sources.

Two metadata-only candidate corpora are compared against the archived source
manifests. Neither filename similarity nor byte identity licenses or verifies
a cheat, and no cheat payload is fetched or stored by this audit.
"""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

from audit_external_archive_overlap import load_archived
from audit_historic_libretro import HIST_REV
from export_historic_libretro import build_live as build_historical, FORMAT as HIST_FORMAT
from external_bulk import ROOT, load as load_external

FORMAT = "cheatarium-historical-external-exact-blob-overlap-v1"
SHA = re.compile(r"[0-9a-f]{40}\Z")


def compile_report(historical, externals, archived_blobs):
    if (historical.get("format") != HIST_FORMAT
            or historical.get("snapshot_commit") != HIST_REV
            or historical.get("metadata_only") is not True
            or historical.get("code_payloads_included") is not False
            or historical.get("rights_status") != "link_only_rights_review"
            or historical.get("archived") is not False):
        raise ValueError("Historical corpus or rights status not pinned")
    rows = historical.get("files")
    if not isinstance(rows, list) or len(rows) != historical.get("original_file_occurrences"):
        raise ValueError("Historical occurrence count changed")

    ext_by_sha = defaultdict(list)
    ext_sources = set()
    ext_paths = set()
    ext_count = 0
    for source in externals:
        source_id = source.get("id")
        platform = source.get("platform")
        if (not isinstance(source_id, str) or not source_id or source_id in ext_sources
                or not isinstance(platform, str) or not platform
                or source.get("rights_status") != "link_only_rights_review"):
            raise ValueError("External source metadata/rights invalid")
        ext_sources.add(source_id)
        for path, sha, size in source["files"]:
            if (not isinstance(path, str) or not path or
                    (source_id, path) in ext_paths or not SHA.fullmatch(sha)
                    or not isinstance(size, int) or isinstance(size, bool) or size < 0):
                raise ValueError("Malformed or repeated external reference")
            ext_paths.add((source_id, path))
            ext_by_sha[sha].append((source_id, platform, path, size))
            ext_count += 1

    known_archive = set(archived_blobs)
    if any(not isinstance(sha, str) or not SHA.fullmatch(sha)
           for sha in known_archive):
        raise ValueError("Invalid archived blob SHA")
    missing_external_blobs = set(ext_by_sha) - known_archive
    historic_shas = set()
    paths = set()
    per_folder = defaultdict(lambda: {"file_occurrences": 0,
                                      "matching_external_occurrences": 0})
    by_source = defaultdict(lambda: {"external_reference_occurrences": 0,
                                     "historical_file_occurrences": 0,
                                     "shared_distinct_blobs": set()})
    matched_rows = []
    overlap_distinct = set()
    total = matched = 0

    for row in rows:
        path, sha, size, folder = (row.get("source_path"), row.get("git_blob_sha"),
                                    row.get("bytes"), row.get("upstream_folder"))
        if (not isinstance(path, str) or not path.startswith("cht/")
                or path in paths or not SHA.fullmatch(sha)
                or not isinstance(size, int) or isinstance(size, bool) or size < 0
                or not isinstance(folder, str) or folder != path[4:].split("/")[0]):
            raise ValueError("Malformed or repeated historical path")
        if sha in known_archive:
            raise ValueError("Historical candidate is already in preserved archive")
        paths.add(path)
        historic_shas.add(sha)
        total += 1
        per_folder[folder]["file_occurrences"] += 1
        found = ext_by_sha.get(sha, [])
        if found:
            matched += 1
            overlap_distinct.add(sha)
            per_folder[folder]["matching_external_occurrences"] += 1
            seen_sources = set()
            for source, platform, other_path, other_size in found:
                if other_size != size:
                    raise ValueError("Identical Git blobs report conflicting sizes")
                stats = by_source[source]
                stats["external_reference_occurrences"] += 1
                stats["shared_distinct_blobs"].add(sha)
                if source not in seen_sources:
                    stats["historical_file_occurrences"] += 1
                    seen_sources.add(source)
            matched_rows.append((path, sha, found))

    if total != historical.get("original_file_occurrences") or len(historic_shas) != historical.get("distinct_git_blob_sha1"):
        raise ValueError("Historical title/blob counts fail reconciliation")
    if len(ext_sources) != len(externals) or ext_count != len(ext_paths):
        raise ValueError("External occurrence count fails reconciliation")
    folders = []
    for folder, stats in sorted(per_folder.items()):
        count = stats["file_occurrences"]
        n = stats["matching_external_occurrences"]
        folders.append({
            "folder": folder, **stats,
            "without_external_byte_match": count - n,
        })
    sources = [
        {"source_id": identifier,
         "matching_external_reference_occurrences": stats["external_reference_occurrences"],
         "matching_historical_file_occurrences": stats["historical_file_occurrences"],
         "shared_distinct_blobs": len(stats["shared_distinct_blobs"])}
        for identifier, stats in sorted(by_source.items())
    ]
    samples = []
    for path, sha, found in sorted(matched_rows)[:12]:
        for source, platform, other_path, _ in sorted(found)[:2]:
            samples.append({
                "historical_path": path, "git_blob_sha": sha,
                "external_source_id": source, "external_platform": platform,
                "external_path": other_path,
            })
    report = {
        "schema_version": 1, "format": FORMAT,
        "method": "pinned-metadata-git-blob-sha1-only",
        "metadata_only": True, "cheat_bodies_included": False,
        "historical_rights_confirmed": False, "external_rights_confirmed": False,
        "proves_unique_games_or_cheat_effects": False,
        "proves_gameplay_compatibility": False,
        "historical_commit": HIST_REV,
        "historical_unarchived_file_occurrences": total,
        "historical_unarchived_distinct_blobs": len(historic_shas),
        "other_external_reference_occurrences": ext_count,
        "other_external_source_catalogs": len(externals),
        "other_external_distinct_blobs": len(ext_by_sha),
        "other_external_distinct_blobs_absent_from_archive": len(missing_external_blobs),
        "historical_file_occurrences_with_external_byte_match": matched,
        "historical_file_occurrences_without_external_byte_match": total - matched,
        "historical_distinct_blobs_with_external_byte_match": len(overlap_distinct),
        "historical_distinct_blobs_without_external_byte_match":
            len(historic_shas) - len(overlap_distinct),
        "combined_unarchived_distinct_blobs":
            len(missing_external_blobs | historic_shas),
        "combined_unarchived_blobs_are_not_distinct_games_or_cheats": True,
        "historical_folders": folders,
        "external_sources_with_matching_bytes": sources,
        "bounded_exact_blob_examples": samples,
    }
    if (sum(x["file_occurrences"] for x in folders) != total
            or sum(x["matching_external_occurrences"] for x in folders) != matched
            or report["combined_unarchived_distinct_blobs"] !=
               len(missing_external_blobs) + len(historic_shas) - len(overlap_distinct)):
        raise ValueError("Cross-corpus overlap counts fail reconciliation")
    return report


def build_live():
    historical = build_historical()
    externals = load_external(ROOT)
    archived_blobs = {sha for sha, _, _ in load_archived(ROOT)}
    report = compile_report(historical, externals, archived_blobs)
    published = json.loads((ROOT / "generated/v1/reports/external-vs-archive-blobs.json").read_text("utf-8"))
    if (report["other_external_reference_occurrences"] !=
            published["external_reference_files"]
            or report["other_external_distinct_blobs_absent_from_archive"] !=
            published["external_distinct_git_blobs_without_archive_byte_match"]):
        raise ValueError("Published external/archive audit contradicts combined overlap")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path,
                        help="Write deterministic JSON report (metadata only)")
    args = parser.parse_args()
    result = build_live()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", "utf-8")
    print("OK: combined distinct unarchived Git blobs:",
          result["combined_unarchived_distinct_blobs"],
          "(not unique games or cheats)")


if __name__ == "__main__":
    main()
