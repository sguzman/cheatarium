#!/usr/bin/env python3
"""Audit byte-exact historical Libretro fork differences; never copy cheat bodies.

Fetches only Git tree metadata for two immutable revisions. The published report
compares the 2019 Emupedia fork with the archived Libretro baseline and the
already imported BitHeX delta. Other Cheatarium sources are not compared.
"""
import argparse
import json
import re
import urllib.request
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "generated/v1/reports/libretro-historical-fork-coverage.json"
FORMAT = "cheatarium-libretro-historical-fork-coverage-v1"
HIST_REPO = "Emupedia/emupedia-docs-libretro-database"
HIST_REV = "c68abd0536adbb595bb833594de237e96626020d"
HIST_TREE = "f56cd8051f3643998a182493c7ea569f62ba7fd3"
BASE_REPO = "libretro/libretro-database"
BASE_REV = "fbeefcb46c2e1b20a7e2945f34a694a41b2d6f90"
BASE_TREE = "82732f21cd5b083ad7b8625fa7c8ca1ca920b6b1"
BITHEX_REV = "837bc5b7b4b872f5b48c6b2e683819ec02db23b8"
SHA = re.compile(r"[0-9a-f]{40}\Z")


def parse_cheat_tree(payload, expected_tree):
    """Return strict (path, blob SHA, size) rows from a complete Git tree."""
    if payload.get("sha") != expected_tree or payload.get("truncated") is not False:
        raise ValueError("Wrong or incomplete pinned Git tree")
    entries = payload.get("tree")
    if not isinstance(entries, list):
        raise ValueError("Git tree entries missing")
    result = []
    seen = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("Invalid Git tree entry")
        path = entry.get("path")
        if not isinstance(path, str):
            raise ValueError("Git tree path is missing")
        if entry.get("type") != "blob" or not path.endswith(".cht"):
            continue
        parts = path.split("/")
        blob = entry.get("sha")
        size = entry.get("size")
        if (not path or path.startswith("/") or "\\" in path
                or any(p in ("", ".", "..") for p in parts)
                or entry.get("mode") not in ("100644", "100755")
                or not isinstance(blob, str) or not SHA.fullmatch(blob)
                or not isinstance(size, int) or isinstance(size, bool) or size < 0
                or path in seen):
            raise ValueError("Unsafe, duplicate, or malformed cheat tree entry")
        seen.add(path)
        result.append((path, blob, size))
    return sorted(result)


def load_bithex(root=ROOT):
    doc = json.loads((root / "sources/bithex-libretro-delta.json").read_text("utf-8"))
    if doc.get("id") != "bithex-libretro-delta" or doc.get("snapshot_commit") != BITHEX_REV:
        raise ValueError("BitHeX snapshot changed; re-audit")
    rows = doc.get("files")
    if not isinstance(rows, list):
        raise ValueError("BitHeX source files missing")
    cheats = [item for item in rows if item.get("upstream_path", "").endswith(".cht")]
    if len(cheats) != 1125 or any(
        not isinstance(item.get("git_blob_sha"), str)
        or not SHA.fullmatch(item["git_blob_sha"]) for item in cheats
    ):
        raise ValueError("BitHeX import inventory changed; re-audit")
    return {item["git_blob_sha"] for item in cheats}


def compile_report(historic, baseline, bithex_blobs, root_license_present=False):
    old_paths = {p for p, _, _ in historic}
    new_paths = {p: sha for p, sha, _ in baseline}
    if len(old_paths) != len(historic) or len(new_paths) != len(baseline):
        raise ValueError("Duplicate upstream path")
    modern_blobs = {sha for _, sha, _ in baseline}
    exact_paths = sum(new_paths.get(p) == sha for p, sha, _ in historic)
    changed_paths = sum(p in new_paths and new_paths[p] != sha for p, sha, _ in historic)
    missing_modern = [row for row in historic if row[1] not in modern_blobs]
    found_bithex = [row for row in missing_modern if row[1] in bithex_blobs]
    remaining = [row for row in missing_modern if row[1] not in bithex_blobs]
    folders = Counter(p.split("/")[0] for p, _, _ in remaining)
    report = {
        "schema_version": 1,
        "format": FORMAT,
        "method": "pinned-git-tree-exact-blob-sha1",
        "historical_repository": "https://github.com/" + HIST_REPO,
        "historical_commit": HIST_REV,
        "historical_cheat_tree_sha": HIST_TREE,
        "baseline_repository": "https://github.com/" + BASE_REPO,
        "baseline_commit": BASE_REV,
        "baseline_cheat_tree_sha": BASE_TREE,
        "additional_archived_source": "bithex-libretro-delta",
        "additional_archived_commit": BITHEX_REV,
        "historical_root_license_file_present": bool(root_license_present),
        "historical_cheat_file_occurrences": len(historic),
        "baseline_cheat_file_occurrences": len(baseline),
        "same_path_same_blob_occurrences": exact_paths,
        "same_path_different_blob_occurrences": changed_paths,
        "historical_paths_absent_from_baseline": len(historic) - exact_paths - changed_paths,
        "historical_occurrences_with_blob_in_baseline":
            len(historic) - len(missing_modern),
        "historical_occurrences_with_blob_absent_from_baseline": len(missing_modern),
        "baseline_absent_occurrences_already_in_bithex": len(found_bithex),
        "candidate_occurrences_absent_from_both": len(remaining),
        "candidate_distinct_blobs_absent_from_both": len({sha for _, sha, _ in remaining}),
        "candidate_occurrences_by_historical_folder": dict(sorted(folders.items())),
        "candidate_examples": [
            {"upstream_path": path, "git_blob_sha": sha, "bytes": size}
            for path, sha, size in remaining[:12]
        ],
        "source_payloads_included": False,
        "compared_against_entire_cheatarium_archive": False,
        "historical_code_redistribution_rights_confirmed": False,
        "proves_distinct_cheats_or_games": False,
        "proves_gameplay_compatibility": False,
    }
    validate_report(report)
    return report


def validate_report(doc):
    if (doc.get("schema_version") != 1 or doc.get("format") != FORMAT
            or doc.get("historical_commit") != HIST_REV
            or doc.get("historical_cheat_tree_sha") != HIST_TREE
            or doc.get("baseline_commit") != BASE_REV
            or doc.get("baseline_cheat_tree_sha") != BASE_TREE
            or doc.get("additional_archived_commit") != BITHEX_REV
            or doc.get("source_payloads_included") is not False
            or doc.get("compared_against_entire_cheatarium_archive") is not False
            or doc.get("historical_code_redistribution_rights_confirmed") is not False
            or doc.get("proves_distinct_cheats_or_games") is not False
            or doc.get("proves_gameplay_compatibility") is not False):
        raise ValueError("Report source pin, provenance boundary, or rights claim invalid")
    n = doc["historical_cheat_file_occurrences"]
    base_absent = doc["historical_occurrences_with_blob_absent_from_baseline"]
    remains = doc["candidate_occurrences_absent_from_both"]
    if (min(n, remains, base_absent) < 0
            or doc["same_path_same_blob_occurrences"]
            + doc["same_path_different_blob_occurrences"]
            + doc["historical_paths_absent_from_baseline"] != n
            or doc["historical_occurrences_with_blob_in_baseline"] + base_absent != n
            or doc["baseline_absent_occurrences_already_in_bithex"] + remains != base_absent
            or sum(doc["candidate_occurrences_by_historical_folder"].values()) != remains
            or not 0 <= doc["candidate_distinct_blobs_absent_from_both"] <= remains):
        raise ValueError("Report counts do not reconcile")
    examples = doc.get("candidate_examples")
    if not isinstance(examples, list) or len(examples) > 12 or len(examples) > remains:
        raise ValueError("Invalid bounded historical examples")
    for example in examples:
        if (not isinstance(example, dict)
                or not isinstance(example.get("upstream_path"), str)
                or not isinstance(example.get("git_blob_sha"), str)
                or not SHA.fullmatch(example["git_blob_sha"])
                or not isinstance(example.get("bytes"), int)
                or example["bytes"] < 0):
            raise ValueError("Invalid historical example")


def fetch_tree(repo, revision):
    url = f"https://api.github.com/repos/{repo}/git/trees/{revision}"
    request = urllib.request.Request(url, headers={
        "Accept": "application/vnd.github+json",
        "User-Agent": "Cheatarium-historical-metadata-audit",
    })
    with urllib.request.urlopen(request, timeout=90) as response:
        return json.load(response)


def build_live():
    root = fetch_tree(HIST_REPO, HIST_REV)
    if root.get("truncated") is not False or not isinstance(root.get("tree"), list):
        raise ValueError("Historical root inventory is incomplete")
    has_license = any(
        x.get("type") == "blob" and x.get("path") in ("LICENSE", "LICENSE.md", "COPYING")
        for x in root["tree"]
    )
    historic = parse_cheat_tree(fetch_tree(HIST_REPO, HIST_TREE), HIST_TREE)
    baseline = parse_cheat_tree(fetch_tree(BASE_REPO, BASE_TREE), BASE_TREE)
    return compile_report(historic, baseline, load_bithex(), has_license)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Refresh pinned report via GitHub API")
    mode.add_argument("--check", action="store_true", help="Compare report to pinned GitHub API")
    mode.add_argument("--offline-check", action="store_true", help="Check report consistency without network")
    args = parser.parse_args()
    if args.offline_check:
        validate_report(json.loads(REPORT.read_text("utf-8")))
    else:
        rendered = json.dumps(build_live(), ensure_ascii=False, indent=2) + "\n"
        if args.write:
            REPORT.parent.mkdir(parents=True, exist_ok=True)
            REPORT.write_text(rendered, "utf-8")
        elif REPORT.read_text("utf-8") != rendered:
            raise SystemExit("Pinned historical Libretro fork report is stale or altered")
    print("OK: pinned historical Libretro fork audit")


if __name__ == "__main__":
    main()
