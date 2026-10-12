#!/usr/bin/env python3
"""Export pinned, unarchived historical .cht paths as rights-blocked references.

Uses only Git tree metadata, hashes and source manifests. No cheat bodies are
downloaded or published; the inventory is separate from archived cheats.
"""
import argparse
import json
import urllib.parse
from pathlib import Path

from audit_historic_libretro import (
    HIST_REPO, HIST_REV, HIST_TREE, BASE_REPO, BASE_TREE,
    REPORT, fetch_tree, parse_cheat_tree, load_bithex, load_archived,
    validate_report,
)

FORMAT = "cheatarium-historical-libretro-unarchived-links-v1"
STATUS = "link_only_rights_review"


def candidate_rows(historic, baseline, bithex_blobs, archived):
    """Select byte-absent historical occurrences, preserving repeated paths."""
    modern = {sha for _, sha, _ in baseline}
    preserved = modern | set(bithex_blobs) | {sha for sha, _, _ in archived}
    rows = [(path, sha, size) for path, sha, size in historic
            if sha not in preserved]
    return sorted(rows)


def render_catalog(rows, audit):
    """Build reproducible, source-pinned, metadata-only references."""
    validate_report(audit)
    expected = audit["candidate_occurrences_absent_from_entire_archive"]
    distinct_expected = audit["candidate_distinct_blobs_absent_from_entire_archive"]
    if (len(rows) != expected
            or len({sha for _, sha, _ in rows}) != distinct_expected
            or audit["historical_commit"] != HIST_REV
            or audit["historical_cheat_tree_sha"] != HIST_TREE):
        raise ValueError("Historical references and pinned audit do not reconcile")

    records = []
    rumble_label_count = 0
    for path, sha, size in rows:
        if (not path.endswith(".cht") or not isinstance(size, int)
                or size < 0 or not isinstance(sha, str) or len(sha) != 40):
            raise ValueError("Invalid native historical cheat metadata")
        rumble = "(retroarch rumbles)" in path.lower()
        rumble_label_count += rumble
        records.append({
            "source_path": "cht/" + path,
            "git_blob_sha": sha,
            "bytes": size,
            "upstream_folder": path.split("/", 1)[0],
            "rumble_filename_signal": rumble,
            "upstream_url": ("https://github.com/" + HIST_REPO + "/blob/"
                             + HIST_REV + "/" + urllib.parse.quote("cht/" + path,
                                                                  safe="/")),
        })
    return {
        "schema_version": 1,
        "format": FORMAT,
        "source_repository": "https://github.com/" + HIST_REPO,
        "snapshot_commit": HIST_REV,
        "source_cheat_tree_sha": HIST_TREE,
        "metadata_only": True,
        "rights_status": STATUS,
        "code_payloads_included": False,
        "archived": False,
        "gameplay_verified": False,
        "label_based_game_identity_verified": False,
        "rumble_filename_signal_is_not_payload_classification": True,
        "original_file_occurrences": len(records),
        "distinct_git_blob_sha1": distinct_expected,
        "rumble_filename_signal_occurrences": rumble_label_count,
        "other_cht_filename_occurrences": len(records) - rumble_label_count,
        "files": records,
    }


def build_live():
    historic = parse_cheat_tree(
        fetch_tree(HIST_REPO, HIST_TREE, recursive=True), HIST_TREE)
    baseline = parse_cheat_tree(
        fetch_tree(BASE_REPO, BASE_TREE, recursive=True), BASE_TREE)
    archived = load_archived()
    rows = candidate_rows(historic, baseline, load_bithex(), archived)
    audit = json.loads(REPORT.read_text("utf-8"))
    return render_catalog(rows, audit)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path,
                        help="Write deterministic metadata-only JSON")
    args = parser.parse_args()
    catalog = build_live()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", "utf-8")
    print("OK: historical Libretro unarchived metadata references:",
          catalog["original_file_occurrences"],
          "file occurrences; no code bodies")


if __name__ == "__main__":
    main()
