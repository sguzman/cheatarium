#!/usr/bin/env python3
"""Verify pinned original-author rights witnesses against Git *tree metadata*.

Only root Git tree listings are requested. No license text, cheat/patch body,
ROM, binary asset, or emulator file is downloaded or redistributed.
"""
import argparse
import json
import os
import re
import urllib.parse
import urllib.request

from audit_acquisition_rights import build

SHA = re.compile(r"[0-9a-f]{40}\Z")
ROOT_LICENSE = re.compile(r"(?:LICENSE|COPYING|NOTICE)(?:\..*)?\Z", re.I)
REPO = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\Z")


def github_tree(repository, tree_sha):
    uri = urllib.parse.urlparse(repository)
    name = uri.path.lstrip("/")
    if (uri.scheme != "https" or uri.netloc != "github.com"
            or uri.params or uri.query or uri.fragment
            or not REPO.fullmatch(name) or not SHA.fullmatch(tree_sha)):
        raise ValueError("Unsafe external Git repository/tree URL")
    url = "https://api.github.com/repos/" + name + "/git/trees/" + tree_sha
    headers = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "Cheatarium-rights-evidence-metadata-check",
    }
    if os.environ.get("GITHUB_TOKEN"):
        headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
    req = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=30) as stream:
        return json.load(stream)


def verify(report, fetch_tree=github_tree):
    if (report.get("format") != "cheatarium-acquisition-rights-review-v1"
            or report.get("metadata_only") is not True
            or report.get("no_automatic_rights_clearance") is not True
            or report.get("cheat_or_patch_bodies_included") is not False):
        raise ValueError("Invalid rights evidence report")
    checked = 0
    license_scopes = 0
    for record in report["reviewed_sources"]:
        tree = fetch_tree(record["repository"], record["pinned_git_tree"])
        if (tree.get("sha") != record["pinned_git_tree"]
                or tree.get("truncated")
                or not isinstance(tree.get("tree"), list)):
            raise ValueError("Incomplete or changed pinned root Git tree")
        blobs = {}
        for entry in tree["tree"]:
            p = entry.get("path")
            if (entry.get("type") == "blob" and isinstance(p, str)
                    and "/" not in p):
                if p in blobs:
                    raise ValueError("Duplicate root Git file path")
                blobs[p] = entry["sha"]
        root_licenses = sorted(
            name for name in blobs if ROOT_LICENSE.fullmatch(name))
        if root_licenses != sorted(record["root_license_paths_observed"]):
            raise ValueError("Observed root license inventory differs from pinned tree: "
                             + record["source_id"])
        for doc in record["documentary_evidence"]:
            path = doc["upstream_path"]
            # Documentary witnesses must be root files in this initial review.
            if "/" in path or blobs.get(path) != doc["git_blob_sha"]:
                raise ValueError("Pinned rights witness Git blob mismatch: "
                                 + record["source_id"] + "/" + path)
            checked += 1
        license_scopes += len(root_licenses)
        if record["redistribution_cleared"] is not False:
            raise ValueError("Rights review cannot silently promote code content")
    if report["source_reviews_with_pinned_evidence"] != len(report["reviewed_sources"]):
        raise ValueError("Evidence count mismatch")
    return {"source_revisions_checked": len(report["reviewed_sources"]),
            "documentary_git_blobs_checked": checked,
            "root_license_file_occurrences": license_scopes,
            "cheat_code_payloads_downloaded": 0,
            "original_author_rights_cleared": 0}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    modes = ap.add_mutually_exclusive_group(required=True)
    modes.add_argument("--verify-upstream", action="store_true")
    modes.add_argument("--offline", action="store_true")
    args = ap.parse_args()
    report = build()
    if args.offline:
        print("OK: rights report metadata and priority counts reconcile offline")
    else:
        print(json.dumps(verify(report), indent=2))


if __name__ == "__main__":
    main()
