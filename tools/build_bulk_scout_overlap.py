#!/usr/bin/env python3
"""Measure pinned bulk scouts against every preserved and link-only Git blob.

All inputs are local, frozen manifests. Git blob SHA-1 establishes only exact
source-byte identity, never code equivalence, ownership, or functionality.
No ROMs, emulator execution, ZIP extraction, or third-party payload reads.
"""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

from audit_bulk_source_scout import FILE as SCOUT_FILE, verify
from audit_external_archive_overlap import load_archived
from external_bulk import ROOT, load, SCOUT_GROUPS

REL = Path("generated/v1/reports/bulk-scout-exact-byte-gaps.json")
FORMAT = "cheatarium-bulk-scout-exact-byte-gaps-v1"
SHA = re.compile(r"[0-9a-f]{40}\Z")


def compile_report(scout_sources, archived_rows, external_sources):
    archive = defaultdict(list)
    for sha, source_id, path in archived_rows:
        if not isinstance(sha, str) or not SHA.fullmatch(sha):
            raise ValueError("Invalid archived blob identity")
        archive[sha].append((source_id, path))
    for witnesses in archive.values():
        witnesses.sort()

    external = defaultdict(set)
    reference_count = 0
    for src in external_sources:
        for path, sha, size in src["files"]:
            if (not isinstance(sha, str) or not SHA.fullmatch(sha)
                    or not isinstance(size, int) or isinstance(size, bool) or size < 0):
                raise ValueError("Invalid known external blob")
            external[sha].add(src["id"])
            reference_count += 1

    source_results = []
    all_scout = set()
    source_ids = set()
    occurrences = 0
    for source in scout_sources:
        sid = source["id"]
        if (sid in source_ids or not isinstance(sid, str) or not sid
                or source.get("rights_status") != "link_only_rights_review"
                or source.get("reuse_cleared") is not False
                or source.get("source_code_included") is not False):
            raise ValueError("Invalid or duplicated rights-unresolved scout")
        source_ids.add(sid)
        paths = set()
        blobs = set()
        matches_archive = matches_external = matches_either = 0
        overlap_samples = []
        gap_samples = []
        for row in source["files"]:
            if not isinstance(row, list) or len(row) != 3:
                raise ValueError("Scout record contains unexpected fields")
            path, sha, size = row
            if (not isinstance(path, str) or not path or path in paths
                    or path.startswith("/") or "\\" in path
                    or any(part in ("", ".", "..") for part in path.split("/"))
                    or not isinstance(sha, str) or not SHA.fullmatch(sha)
                    or not isinstance(size, int) or isinstance(size, bool) or size < 0):
                raise ValueError("Invalid scout path, blob or file length")
            paths.add(path)
            blobs.add(sha)
            in_archive = sha in archive
            in_external = sha in external
            matches_archive += in_archive
            matches_external += in_external
            matches_either += in_archive or in_external
            if in_archive or in_external:
                if len(overlap_samples) < 3:
                    overlap_samples.append({
                        "path": path, "git_blob_sha": sha,
                        "archived_source_id": archive[sha][0][0] if in_archive else None,
                        "external_source_ids": sorted(external[sha]) if in_external else [],
                    })
            elif len(gap_samples) < 3:
                gap_samples.append({"path": path, "git_blob_sha": sha})
        if (source["selected_file_occurrences"] != len(paths)
                or source["distinct_git_blobs"] != len(blobs)):
            raise ValueError("Scout totals disagree with original pinned inventory")
        occurrences += len(paths)
        all_scout.update(blobs)
        source_results.append({
            "source_id": sid,
            "rights_status": "link_only_rights_review",
            "file_occurrences": len(paths),
            "distinct_source_git_blobs": len(blobs),
            "repeated_byte_occurrences_within_source": len(paths) - len(blobs),
            "occurrences_matching_archived_original_bytes": matches_archive,
            "occurrences_matching_existing_external_bytes": matches_external,
            "occurrences_matching_either_known_corpus": matches_either,
            "distinct_blobs_matching_archived_original_bytes": len(blobs & archive.keys()),
            "distinct_blobs_matching_existing_external_bytes": len(blobs & external.keys()),
            "distinct_blobs_not_in_either_known_corpus":
                len(blobs - archive.keys() - external.keys()),
            "known_byte_match_examples": overlap_samples,
            "no_known_byte_match_examples": gap_samples,
        })

    if occurrences != sum(x["file_occurrences"] for x in source_results):
        raise ValueError("Scout occurrence totals failed reconciliation")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "pinned-original-git-blob-sha1-only",
        "metadata_only": True,
        "source_payloads_included": False,
        "rights_to_redistribute_established": False,
        "new_games_or_cheat_effects_verified": False,
        "gameplay_compatibility_verified": False,
        "coverage_universe": "pinned preserved originals and preexisting external catalogs only",
        "archived_original_file_occurrences": len(archived_rows),
        "existing_external_reference_occurrences": reference_count,
        "scout_source_count": len(scout_sources),
        "scout_source_file_occurrences": occurrences,
        "scout_distinct_git_blobs": len(all_scout),
        "scout_distinct_git_blobs_matching_archive": len(all_scout & archive.keys()),
        "scout_distinct_git_blobs_matching_existing_external":
            len(all_scout & external.keys()),
        "scout_distinct_git_blobs_not_in_either_known_corpus":
            len(all_scout - archive.keys() - external.keys()),
        "sources": source_results,
    }


def build(root=ROOT):
    doc = json.loads((root / "research/bulk-source-scout-v1.json").read_text("utf-8"))
    verify(doc, root)
    # The scouts are now searchable as external references. Keep the original
    # pre-onboarding byte-gap baseline, excluding those same nine partitions
    # from self-comparison; otherwise every candidate becomes its own match.
    baseline = [s for s in load(root) if s["id"] not in SCOUT_GROUPS]
    report = compile_report(doc["sources"], load_archived(root), baseline)
    if (report["scout_source_file_occurrences"] != doc["source_file_occurrences"]
            or report["scout_distinct_git_blobs"] != doc["distinct_git_blobs_in_source_union"]):
        raise ValueError("Scout byte coverage does not reconcile with pinned source inventory")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = build()
    serialized = json.dumps(data, indent=2, ensure_ascii=False) + "\n"
    destination = ROOT / REL
    if args.write:
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(serialized, "utf-8")
    elif not destination.is_file() or destination.read_text("utf-8") != serialized:
        raise SystemExit("Pinned bulk scout exact-byte gap audit is stale or altered")
    print(f"OK: {data['scout_distinct_git_blobs_not_in_either_known_corpus']} "
          f"byte-distinct scout blobs absent from known archived and external catalogs "
          f"({data['scout_source_file_occurrences']} original scout paths)")


if __name__ == "__main__":
    main()
