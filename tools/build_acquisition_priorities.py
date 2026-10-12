#!/usr/bin/env python3
"""Rank rights-unresolved bulk source catalogs by exact-byte preservation gaps.

This is a rights-review *queue*, never an approval to copy cheat bodies.
All records are pinned path / Git blob SHA metadata. A distinct blob is not
a unique game, playable build, executable effect, or source authorship claim.
"""
import argparse
import json
import re
from collections import defaultdict
from pathlib import Path

from audit_external_archive_overlap import load_archived
from external_bulk import ROOT, classify, load

REPORT = Path("generated/v1/reports/acquisition-priorities.json")
FORMAT = "cheatarium-acquisition-priorities-v1"
SHA = re.compile(r"[a-f0-9]{40}\Z")
STATUS = "link_only_rights_review"


def compile_report(archived_rows, external_sources, classifier=classify):
    archived = {sha for sha, _, _ in archived_rows}
    if any(not isinstance(x, str) or not SHA.fullmatch(x) for x in archived):
        raise ValueError("Malformed archived original Git blob identity")
    global_owners = defaultdict(set)
    blobs_by_platform = defaultdict(set)
    ids = set()
    total_occurrences = 0
    all_bytes = {}
    groups = []
    file_kinds_by_hash = defaultdict(set)
    file_kinds_by_platform_hash = defaultdict(set)
    for source in external_sources:
        sid = source.get("id")
        platform = source.get("platform")
        if (not isinstance(sid, str) or not sid or sid in ids
                or not isinstance(platform, str) or not platform
                or source.get("rights_status") != STATUS
                or not isinstance(source.get("files"), list)):
            raise ValueError("Duplicate or rights-unresolved source metadata changed")
        ids.add(sid)
        seen_paths = set()
        kind_occurrences = {"cheat": 0, "patch": 0}
        hash_kinds = defaultdict(set)
        hashes = set()
        unmatched = set()
        unmatched_paths = []
        archived_occurrences = 0
        for row in source["files"]:
            if not isinstance(row, (list, tuple)) or len(row) != 3:
                raise ValueError("Invalid path/hash/size tuple")
            path, sha, size = row
            if (not isinstance(path, str) or not path or path in seen_paths
                    or not isinstance(sha, str) or not SHA.fullmatch(sha)
                    or not isinstance(size, int) or isinstance(size, bool) or size < 0):
                raise ValueError("Invalid or repeated source reference")
            seen_paths.add(path)
            if sha in all_bytes and all_bytes[sha] != size:
                raise ValueError("Identical Git blob reports conflicting byte sizes")
            all_bytes[sha] = size
            kind = classifier(sid, path)
            if kind not in kind_occurrences:
                raise ValueError("Unrecognized source-native cheat/patch file kind")
            kind_occurrences[kind] += 1
            hash_kinds[sha].add(kind)
            global_owners[sha].add(sid)
            blobs_by_platform[platform].add(sha)
            hashes.add(sha)
            total_occurrences += 1
            if sha in archived:
                archived_occurrences += 1
            else:
                unmatched.add(sha)
                unmatched_paths.append((path, sha, kind))
                file_kinds_by_hash[sha].add(kind)
                file_kinds_by_platform_hash[(platform, sha)].add(kind)
        groups.append({
            "source_id": sid,
            "platform": platform,
            "rights_status": STATUS,
            "original_file_occurrences": len(seen_paths),
            "cheat_labeled_occurrences": kind_occurrences["cheat"],
            "patch_labeled_occurrences": kind_occurrences["patch"],
            "occurrences_already_archived_exact_bytes": archived_occurrences,
            "occurrences_without_archived_exact_bytes": len(seen_paths) - archived_occurrences,
            "_distinct_all": hashes,
            "_distinct_unarchived": unmatched,
            "_hash_kinds": hash_kinds,
            "_unmatched_paths": unmatched_paths,
        })

    per_source = []
    for group in groups:
        novel = group.pop("_distinct_unarchived")
        all_blobs = group.pop("_distinct_all")
        kinds = group.pop("_hash_kinds")
        paths = group.pop("_unmatched_paths")
        exclusive = {sha for sha in novel if len(global_owners[sha]) == 1}
        shared = novel - exclusive
        cheat = {sha for sha in novel if "cheat" in kinds[sha]}
        patch = {sha for sha in novel if "patch" in kinds[sha]}
        exclusive_cheat = exclusive & cheat
        witnesses = [
            {"path": path, "git_blob_sha": sha}
            for path, sha, kind in sorted(paths)
            if kind == "cheat" and sha in exclusive_cheat
        ][:3]
        per_source.append({
            **group,
            "distinct_blob_sha1": len(all_blobs),
            "distinct_unarchived_blob_sha1": len(novel),
            "distinct_unarchived_cheat_labeled_blobs": len(cheat),
            "distinct_unarchived_patch_labeled_blobs": len(patch),
            "exclusive_unarchived_blob_sha1": len(exclusive),
            "exclusive_unarchived_cheat_labeled_blobs": len(exclusive_cheat),
            "shared_with_other_external_sources_unarchived_blobs": len(shared),
            "sample_exclusive_cheat_source_paths": witnesses,
        })

    per_source.sort(key=lambda r: (
        -r["exclusive_unarchived_cheat_labeled_blobs"],
        -r["distinct_unarchived_cheat_labeled_blobs"],
        -r["exclusive_unarchived_blob_sha1"], r["source_id"]))
    per_platform = []
    for platform, hashes in sorted(blobs_by_platform.items()):
        novel = hashes - archived
        labeled_cheat = {sha for sha in novel if "cheat" in file_kinds_by_platform_hash[(platform, sha)]}
        labeled_patch = {sha for sha in novel if "patch" in file_kinds_by_platform_hash[(platform, sha)]}
        platform_sources = [r for r in per_source if r["platform"] == platform]
        per_platform.append({
            "platform": platform,
            "source_catalogs": len(platform_sources),
            "file_occurrences": sum(r["original_file_occurrences"] for r in platform_sources),
            "distinct_source_blobs": len(hashes),
            "distinct_blobs_absent_from_archive": len(novel),
            "distinct_cheat_labeled_blobs_absent_from_archive": len(labeled_cheat),
            "distinct_patch_labeled_blobs_absent_from_archive": len(labeled_patch),
        })
    per_platform.sort(key=lambda r: (
        -r["distinct_cheat_labeled_blobs_absent_from_archive"], r["platform"]))

    all_references = set(global_owners)
    unarchived = all_references - archived
    cheat_labeled = {sha for sha in unarchived if "cheat" in file_kinds_by_hash[sha]}
    patch_labeled = {sha for sha in unarchived if "patch" in file_kinds_by_hash[sha]}
    exclusive_hashes = {sha for sha in unarchived if len(global_owners[sha]) == 1}
    report = {
        "schema_version": 1, "format": FORMAT,
        "method": "pinned-source-original-git-blob-sha1-only",
        "metadata_only": True,
        "source_rights_cleared_for_import": False,
        "unique_games_or_effects_established": False,
        "gameplay_compatibility_verified": False,
        "filename_kind_is_semantic_cheat_verification": False,
        "source_file_occurrences": total_occurrences,
        "source_catalogs": len(external_sources),
        "platforms_count": len(blobs_by_platform),
        "all_external_distinct_blob_sha1": len(all_references),
        "distinct_blobs_matching_archived_bytes": len(all_references & archived),
        "distinct_blobs_absent_from_archive": len(unarchived),
        "distinct_cheat_labeled_blobs_absent_from_archive": len(cheat_labeled),
        "distinct_patch_labeled_blobs_absent_from_archive": len(patch_labeled),
        "blobs_labeled_both_cheat_and_patch": len(cheat_labeled & patch_labeled),
        "exclusive_unarchived_blobs_in_one_source": len(exclusive_hashes),
        "shared_unarchived_blobs_in_multiple_sources": len(unarchived - exclusive_hashes),
        "ranking_rule": "exclusive-unarchived-cheat-blobs-desc;unarchived-cheat-blobs-desc;exclusive-blobs-desc;source-id",
        "interpretation": (
            "Rights review priority, not a license grant, game count, runnable cheat count, "
            "build compatibility claim, or permission to import."
        ),
        "sources": per_source, "platforms": per_platform,
    }
    if (sum(r["original_file_occurrences"] for r in per_source) != total_occurrences
            or sum(r["file_occurrences"] for r in per_platform) != total_occurrences
            or sum(r["exclusive_unarchived_blob_sha1"] for r in per_source)
               != len(exclusive_hashes)
            or len(cheat_labeled | patch_labeled) != len(unarchived)
            or sum(r["distinct_unarchived_blob_sha1"] for r in per_source) <
               len(unarchived)):
        raise ValueError("Acquisition gap aggregation failed to reconcile")
    return report


def build(root=ROOT):
    report = compile_report(load_archived(root), load(root))
    audited = json.loads((root / "generated/v1/reports/external-vs-archive-blobs.json").read_text("utf-8"))
    overlap = json.loads((root / "generated/v1/reports/external-overlap.json").read_text("utf-8"))
    if (report["source_file_occurrences"] != audited["external_reference_files"]
            or report["all_external_distinct_blob_sha1"] != audited["external_distinct_git_blobs"]
            or report["distinct_blobs_matching_archived_bytes"] !=
               audited["external_distinct_git_blobs_matching_archive"]
            or report["distinct_blobs_absent_from_archive"] !=
               audited["external_distinct_git_blobs_without_archive_byte_match"]
            or report["source_catalogs"] != overlap["source_catalogs"]
            or report["platforms_count"] != overlap["external_platforms"]
            or report["source_file_occurrences"] != overlap["reference_files"]):
        raise ValueError("Acquisition priorities disagree with published byte-identity audits")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    target = ROOT / REPORT
    if args.write:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(rendered, "utf-8")
    elif not target.is_file() or target.read_text("utf-8") != rendered:
        raise SystemExit("Pinned acquisition priority report is stale or altered")
    result = json.loads(rendered)
    print(f"OK: {result['source_catalogs']} rights-blocked catalogs; "
          f"{result['distinct_blobs_absent_from_archive']} byte-distinct "
          "unarchived sources (not new games or rights-cleared cheats)")


if __name__ == "__main__":
    main()
