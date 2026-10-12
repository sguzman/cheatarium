#!/usr/bin/env python3
"""Audit two RetroArch forks against three pinned Libretro-family cheat corpora.

All inputs are Git *tree metadata* (paths, SHA-1 blob IDs and byte lengths).
No original cheat code, archive body or executable content is fetched.
Identical bytes are not independent games, cheat effects, or license grants.
"""
import argparse
import json
import os
import re
import urllib.request
from pathlib import Path

from audit_historic_libretro import (BASE_REPO, BASE_TREE, HIST_REPO,
                                      HIST_TREE, load_bithex, parse_cheat_tree)

ROOT = Path(__file__).resolve().parents[1]
FILE = ROOT / "research/retroarch-mirror-source-gaps-v1.json"
FORMAT = "cheatarium-retroarch-pinned-mirror-byte-gaps-v1"
SOURCES = [
    ("gcmcom-libretro-legacy", "gcmcom/libretro-database",
     "4667ad6247352f57d44f2ad7b36f19f3e804aba9",
     "73797b1c0be7fa1d6b34fd14919fcf4f4d194877", "cht/"),
    ("smoker1-retroarch-174", "Smoker1/RetroArch-174-and-above-Cheats",
     "d07b731562d3a68d089dc0a1ba62311b65a1d312",
     "d07b731562d3a68d089dc0a1ba62311b65a1d312", ""),
]
EXPECTED = {
    "gcmcom-libretro-legacy": (10872, 10861, 11, 11),
    "smoker1-retroarch-174": (518, None, 75, 66),
}
SHA = re.compile(r"[0-9a-f]{40}\Z")


def compile_report(base, historic, bithex, sources):
    known = {sha for _, sha, _ in base} | {sha for _, sha, _ in historic} | set(bithex)
    hist_sha = {sha for _, sha, _ in historic}
    if (len(base) != 27915 or len(historic) != 15714
            or len(set(bithex)) != 1113):
        raise ValueError("Pinned Libretro, Emupedia, or BitHeX baseline changed")
    records = []
    union = set()
    for ident, repo, commit, tree_sha, prefix in SOURCES:
        rows = sources.get(ident)
        if rows is None:
            raise ValueError("Missing pinned fork source")
        if len(rows) != EXPECTED[ident][0] or len({p for p, _, _ in rows}) != len(rows):
            raise ValueError("Pinned source file count or paths changed")
        matched_hist = sum(sha in hist_sha for _, sha, _ in rows)
        unmatched = sorted((p, sha, size) for p, sha, size in rows if sha not in known)
        distinct = {sha for _, sha, _ in unmatched}
        total, hist_expected, missing_expected, distinct_expected = EXPECTED[ident]
        if (len(unmatched) != missing_expected or len(distinct) != distinct_expected
                or (hist_expected is not None and matched_hist != hist_expected)):
            raise ValueError("Pinned fork byte comparison changed")
        files = []
        for path, sha, size in unmatched:
            if (not path or path.startswith("/") or "\\" in path
                    or any(part in ("", ".", "..") for part in path.split("/"))
                    or not isinstance(size, int) or isinstance(size, bool) or size < 0
                    or not SHA.fullmatch(sha)):
                raise ValueError("Invalid or unsafe original metadata-only witness")
            files.append({
                "source_path": prefix + path,
                "git_blob_sha": sha, "bytes": size,
                "original_folder": path.split("/")[0],
                "pinned_upstream_url": f"https://github.com/{repo}/blob/{commit}/{prefix}{path}",
            })
        union |= distinct
        records.append({
            "id": ident, "repository": f"https://github.com/{repo}",
            "snapshot_commit": commit, "source_git_tree": tree_sha,
            "rights_status": "link_only_rights_review",
            "redistribution_cleared": False,
            "code_payloads_included": False,
            "original_cht_file_occurrences": total,
            "occurrences_matching_historical_exact_bytes": matched_hist,
            "file_occurrences_not_matching_three_corpora": len(unmatched),
            "distinct_blob_sha1_not_matching_three_corpora": len(distinct),
            "unmatched_source_file_metadata": files,
        })
    if len(union) > sum(x["distinct_blob_sha1_not_matching_three_corpora"]
                        for x in records):
        raise ValueError("Inconsistent mirror source union")
    return {
        "schema_version": 1, "format": FORMAT,
        "method": "original-git-tree-sha1-identity-only",
        "metadata_only": True, "cheat_or_archive_bodies_included": False,
        "rights_clearance_asserted": False, "source_licensing_verified": False,
        "gameplay_compatibility_verified": False,
        "distinct_games_or_cheats_established": False,
        "compared_with": [
            {"repository": BASE_REPO, "tree_sha": BASE_TREE, "cht_file_occurrences": len(base)},
            {"repository": HIST_REPO, "tree_sha": HIST_TREE, "cht_file_occurrences": len(historic)},
            {"repository": "BitHeX0R/Cheat-database",
             "tree_sha": "not-required-pinned-original-file-manifest",
             "distinct_known_cheat_blobs": len(bithex)},
        ],
        "source_catalogs_not_added_to_external_search": len(records),
        "file_occurrences_not_matching_three_corpora":
            sum(x["file_occurrences_not_matching_three_corpora"] for x in records),
        "distinct_blob_sha1_in_union_not_matching_three_corpora": len(union),
        "absence_from_all_other_archived_corpora_verified": False,
        "notes": (
            "GCMCom is overwhelmingly a byte-identical subset of the already tracked "
            "Emupedia-era source. These few unshared bytes and Smoker1's remaining "
            "source occurrences are rights-unresolved research leads only; neither "
            "independently verified new games nor permissions to copy source contents."
        ),
        "sources": records,
    }


def fetch_tree(repository, sha):
    if (not SHA.fullmatch(sha) or not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository)):
        raise ValueError("Invalid pinned repository/tree SHA")
    url = f"https://api.github.com/repos/{repository}/git/trees/{sha}?recursive=1"
    headers = {"Accept": "application/vnd.github+json",
               "User-Agent": "Cheatarium-metadata-only-mirror-overlap"}
    if os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=60) as f:
        return json.load(f)


def build_live():
    base = parse_cheat_tree(fetch_tree(BASE_REPO, BASE_TREE), BASE_TREE)
    hist = parse_cheat_tree(fetch_tree(HIST_REPO, HIST_TREE), HIST_TREE)
    forks = {
        ident: parse_cheat_tree(fetch_tree(repo, tree_sha), tree_sha)
        for ident, repo, _, tree_sha, _ in SOURCES
    }
    return compile_report(base, hist, load_bithex(ROOT), forks)


def validate_offline(doc):
    if (doc.get("format") != FORMAT or doc.get("schema_version") != 1
            or doc.get("metadata_only") is not True
            or doc.get("cheat_or_archive_bodies_included") is not False
            or doc.get("rights_clearance_asserted") is not False
            or doc.get("absence_from_all_other_archived_corpora_verified") is not False
            or len(doc.get("sources", [])) != 2):
        raise ValueError("Invalid, overstated or incomplete mirror gap report")
    total = 0
    hashes = set()
    for row, spec in zip(doc["sources"], SOURCES):
        ident, repo, commit, tree_sha, prefix = spec
        allfiles = row.get("unmatched_source_file_metadata")
        if (row.get("id") != ident or row.get("repository") != "https://github.com/" + repo
                or row.get("snapshot_commit") != commit or row.get("source_git_tree") != tree_sha
                or row.get("rights_status") != "link_only_rights_review"
                or row.get("redistribution_cleared") is not False
                or row.get("code_payloads_included") is not False
                or not isinstance(allfiles, list)
                or len(allfiles) != EXPECTED[ident][2]
                or row.get("distinct_blob_sha1_not_matching_three_corpora") != EXPECTED[ident][3]):
            raise ValueError("Fork identity/rights counts changed")
        paths = set()
        for f in allfiles:
            if set(f) != {"source_path", "git_blob_sha", "bytes", "original_folder", "pinned_upstream_url"}:
                raise ValueError("Unexpected data field in source metadata record")
            p, sha, size = f["source_path"], f["git_blob_sha"], f["bytes"]
            if (not isinstance(p, str) or not p.startswith(prefix) or p in paths
                    or not SHA.fullmatch(sha) or not isinstance(size, int)
                    or isinstance(size, bool) or size < 0
                    or f["pinned_upstream_url"] !=
                        f"https://github.com/{repo}/blob/{commit}/{p}"
                    or f["original_folder"] != p.removeprefix(prefix).split("/")[0]):
                raise ValueError("Malformed Git-blob witness")
            paths.add(p)
            hashes.add(sha)
        total += len(allfiles)
    if (doc.get("file_occurrences_not_matching_three_corpora") != total
            or doc.get("distinct_blob_sha1_in_union_not_matching_three_corpora") != len(hashes)):
        raise ValueError("Mirrored source totals fail reconciliation")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    m = ap.add_mutually_exclusive_group(required=True)
    m.add_argument("--offline-check", action="store_true")
    m.add_argument("--verify-upstream", action="store_true")
    m.add_argument("--write", action="store_true")
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    if args.offline_check:
        validate_offline(json.loads(FILE.read_text("utf-8")))
    else:
        live = build_live()
        validate_offline(live)
        if args.verify_upstream and live != json.loads(FILE.read_text("utf-8")):
            raise ValueError("Pinned Git mirror byte identities changed")
        if args.output:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(live, ensure_ascii=False, indent=2) + "\n", "utf-8")
        elif args.write:
            FILE.parent.mkdir(parents=True, exist_ok=True)
            FILE.write_text(json.dumps(live, ensure_ascii=False, indent=2) + "\n", "utf-8")
    print("OK: two pinned mirror inventories checked as link-only metadata")


if __name__ == "__main__":
    main()
