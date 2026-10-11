#!/usr/bin/env python3
"""Reproducible metadata-only inventory for rights-unresolved bulk cheat sources.

The pinned references expose names, Git blob IDs, and byte sizes. This tool
never copies third-party cheat or patch bodies into the Cheatarium archive.
"""
import argparse
import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FOLDER = Path("sources/external-bulk")
SOURCES = {
    "duckstation-chtdb": {
        "repository": "duckstation/chtdb",
        "commit": "e3cce509936261808b971199be3a20e84d0b9482",
        "platform": "ps1", "expected": {"cheat": 4497, "patch": 138},
        "rights_note": "The upstream LICENSE explicitly reserves ownership of cheats and patches to original authors; only processing scripts receive its MIT grant. File paths, hashes and byte sizes are indexed here, not code or patch bodies.",
    },
    "xenia-canary-game-patches": {
        "repository": "xenia-canary/game-patches",
        "commit": "106bb0a40be27b5a2444162562975e4885e1ce74",
        "platform": "xbox-360", "expected": {"patch": 501},
        "rights_note": "No repository-wide LICENSE was present in the pinned tree; patch authorship and redistribution permissions must be reviewed. Patches are not necessarily gameplay cheats. Only file names, hashes and sizes are indexed.",
    },
}
SHA = re.compile(r"^[0-9a-f]{40}$")


def classify(source, path):
    if source == "duckstation-chtdb":
        if re.fullmatch(r"cheats/[^/]+\.cht", path):
            return "cheat"
        if re.fullmatch(r"patches/[^/]+\.cht", path):
            return "patch"
    if source == "xenia-canary-game-patches" and re.fullmatch(r"patches/[^/]+\.patch\.toml", path):
        return "patch"
    return None


def make_index(source, tree):
    spec = SOURCES[source]
    if tree.get("truncated") or not isinstance(tree.get("tree"), list):
        raise ValueError("Git tree incomplete")
    rows = sorted(
        [[item["path"], item["sha"], item["size"]]
         for item in tree["tree"]
         if item.get("type") == "blob" and classify(source, item.get("path", ""))],
        key=lambda row: row[0],
    )
    return {
        "schema_version": 1, "id": source,
        "repository": "https://github.com/" + spec["repository"],
        "snapshot_commit": spec["commit"],
        "git_tree_sha": tree["sha"],
        "platform": spec["platform"],
        "rights_status": "link_only_rights_review",
        "rights_note": spec["rights_note"],
        "filename_scope": "source-native cheat and patch files only; source contents intentionally omitted",
        "file_counts": count_types(source, rows),
        "files": rows,
    }


def count_types(source, rows):
    counts = {}
    for path, _, _ in rows:
        kind = classify(source, path)
        if kind is None:
            raise ValueError("Invalid source-native path: " + path)
        counts[kind] = counts.get(kind, 0) + 1
    return counts


def validate(source, index):
    spec = SOURCES[source]
    if (index.get("schema_version") != 1 or index.get("id") != source
            or index.get("repository") != "https://github.com/" + spec["repository"]
            or index.get("snapshot_commit") != spec["commit"]
            or index.get("platform") != spec["platform"]
            or index.get("rights_status") != "link_only_rights_review"
            or index.get("rights_note") != spec["rights_note"]
            or index.get("filename_scope") != "source-native cheat and patch files only; source contents intentionally omitted"
            or not SHA.fullmatch(index.get("git_tree_sha", ""))):
        raise ValueError(f"Metadata or rights status mismatch for {source}")
    rows = index.get("files")
    if not isinstance(rows, list):
        raise ValueError("Missing source file inventory")
    previous = ""
    for row in rows:
        if not (isinstance(row, list) and len(row) == 3):
            raise ValueError("Unexpected field in a metadata-only file record")
        path, blob, size = row
        if (not isinstance(path, str) or not path.isascii() and "\x00" in path
                or path.startswith("/") or "\\" in path
                or any(seg in ("", ".", "..") for seg in path.split("/"))
                or path <= previous or classify(source, path) is None
                or not isinstance(blob, str) or not SHA.fullmatch(blob)
                or not isinstance(size, int) or isinstance(size, bool) or size < 0):
            raise ValueError(f"Unsafe or invalid record: {row!r}")
        previous = path
    if count_types(source, rows) != spec["expected"] or index.get("file_counts") != spec["expected"]:
        raise ValueError("Pinned source counts changed for " + source)
    if set(index) != {"schema_version", "id", "repository", "snapshot_commit",
                      "git_tree_sha", "platform", "rights_status", "rights_note",
                      "filename_scope", "file_counts", "files"}:
        raise ValueError("Unexpected manifest fields may indicate code payload leakage")


def source_link(index, path):
    encoded = urllib.parse.quote(path, safe="/")
    return index["repository"] + "/blob/" + index["snapshot_commit"] + "/" + encoded


def lookup(indexes, needle, platform=None, limit=20):
    needle = needle.casefold()
    found = []
    for index in indexes:
        if platform and platform != index["platform"]:
            continue
        for path, blob, size in index["files"]:
            if needle not in path.casefold():
                continue
            found.append({"platform": index["platform"], "source": index["id"],
                          "kind": classify(index["id"], path), "path": path,
                          "git_blob_sha": blob, "bytes": size,
                          "upstream_url": source_link(index, path),
                          "redistribution": "link_only_rights_review"})
            if len(found) >= limit:
                return found
    return found


def load(root):
    result = []
    for source in SOURCES:
        doc = json.loads((root / FOLDER / (source + ".json")).read_text("utf-8"))
        validate(source, doc)
        result.append(doc)
    return result


def refresh(root):
    for source, spec in SOURCES.items():
        url = "https://api.github.com/repos/" + spec["repository"] + "/git/trees/" + spec["commit"] + "?recursive=1"
        request = urllib.request.Request(url, headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "Cheatarium-external-bulk-metadata-index",
        })
        with urllib.request.urlopen(request, timeout=60) as response:
            upstream = json.load(response)
        index = make_index(source, upstream)
        validate(source, index)
        dest = root / FOLDER / (source + ".json")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n", "utf-8")
        print(f"Refreshed link-only source {source}: {len(index['files'])} filename/hash records")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--refresh", action="store_true")
    mode.add_argument("--search", metavar="TEXT")
    parser.add_argument("--platform", choices=("ps1", "xbox-360"))
    parser.add_argument("--limit", type=int, default=20)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    if args.limit < 1 or args.limit > 100:
        parser.error("--limit must be in [1, 100]")
    if args.refresh:
        refresh(args.root)
    else:
        indexes = load(args.root)
        if args.check:
            for doc in indexes:
                print(f"VALID metadata only: {doc['id']} {len(doc['files'])} original upstream paths")
        else:
            print(json.dumps({"query": args.search, "results": lookup(indexes, args.search, args.platform, args.limit),
                              "note": "External source links, not archived or independently gameplay-verified codes."}, indent=2))


if __name__ == "__main__":
    main()
