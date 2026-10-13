#!/usr/bin/env python3
"""Reproduce and verify pinned, metadata-only bulk cheat collection scouts.

No cheat bodies, ZIP members or executable payloads are fetched. This audits
source file occurrences and Git SHA-1 blob identities, not working cheats.
"""
import argparse
import json
import os
import re
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILE = ROOT / "research/bulk-source-scout-v1.json"
FORMAT = "cheatarium-bulk-source-scout-v1"
SHA = re.compile(r"[0-9a-f]{40}\Z")
SOURCES = (
    ("eignar17-arcade", "Eignar17/kawaks-cheats-Fba-MAME",
     "9bc73944f2b11fc4719d95ce363744842143a778", 621, 420, 5464781),
    ("riyoway-nintendo", "Riyoway/CheatDB",
     "17938073e898b2774e09cce2153baabfeb07aa10", 7, 7, 191660391),
    ("slade1972-segacd", "Slade1972/SegaCD-Cheats-MiSTer",
     "0b2186db50e14e06617d6317d5ee0529c6d4631c", 51, 48, 81664),
    ("skillercmp-xemu", "SkillerCMP/XEMU-CHEATDB",
     "efdb486fbc95c4b8aa9384f47694124c6f03fa32", 12, 12, 26858),
)

def selected(identifier, path):
    if identifier == "eignar17-arcade":
        return bool(re.fullmatch(r"(?:Final Burn Alpha Cheat Archive/[^/]+\.ini|cheats/[^/]+\.dat)", path))
    if identifier == "riyoway-nintendo":
        return path == "cheatdb.db" or bool(re.fullmatch(
            r"exports/(?:JGeckoU/[^/]+\.xml|JSON/Nintendo/[^/]+\.(?:json|jsonl))", path))
    if identifier == "slade1972-segacd":
        return bool(re.fullmatch(r"Cheats/[^/]+\.zip", path))
    if identifier == "skillercmp-xemu":
        return bool(re.fullmatch(r"Cheats/[^/]+\.txt", path))
    return False

def verify(doc, root=ROOT):
    if (set(doc) != {"schema_version", "format", "method",
                     "cheat_bodies_included", "licensed_to_archive",
                     "game_identity_verified", "cheat_function_verified",
                     "archive_novelty_verified", "source_file_occurrences",
                     "distinct_git_blobs_in_source_union", "source_count",
                     "comparisons", "sources"} or doc["schema_version"] != 1
            or doc["format"] != FORMAT
            or doc["method"] != "pinned-git-tree-metadata-only"
            or any(doc[x] is not False for x in (
                "cheat_bodies_included", "licensed_to_archive",
                "game_identity_verified", "cheat_function_verified",
                "archive_novelty_verified"))
            or doc["source_count"] != len(SOURCES)
            or len(doc["sources"]) != len(SOURCES)):
        raise ValueError("Invalid metadata-only scout report header")
    union = set()
    count = 0
    smoker = json.loads((root / "sources/external-bulk/smoker1-fba-official-ini.json").read_text("utf-8"))
    smoker_blobs = {file[1] for file in smoker["files"]}
    for source, (identifier, repository, commit, total, distinct, byte_total) in zip(doc["sources"], SOURCES):
        if (set(source) != {"id", "repository", "snapshot_commit", "source_tree_sha",
                            "rights_status", "reuse_cleared", "source_code_included",
                            "scope", "rights_note", "selected_file_occurrences",
                            "distinct_git_blobs", "selected_bytes",
                            "matching_smoker1_file_occurrences", "files"}
                or source["id"] != identifier
                or source["repository"] != "https://github.com/" + repository
                or source["snapshot_commit"] != commit
                or not SHA.fullmatch(source["source_tree_sha"])
                or source["rights_status"] != "link_only_rights_review"
                or source["reuse_cleared"] is not False
                or source["source_code_included"] is not False
                or not source["scope"] or not source["rights_note"]
                or not isinstance(source["files"], list)):
            raise ValueError("Invalid source identity, rights or payload assertion")
        last = ""
        blobs = set()
        size_sum = 0
        matching = 0
        for file in source["files"]:
            if not isinstance(file, list) or len(file) != 3:
                raise ValueError("Unexpected source payload")
            path, blob, size = file
            if (not isinstance(path, str) or not selected(identifier, path)
                    or path <= last or "\x00" in path or "\\" in path
                    or any(part in ("", ".", "..") for part in path.split("/"))
                    or not isinstance(blob, str) or not SHA.fullmatch(blob)
                    or not isinstance(size, int) or isinstance(size, bool) or size < 0):
                raise ValueError("Malformed source Git tree record")
            last = path
            blobs.add(blob)
            union.add(blob)
            size_sum += size
            matching += blob in smoker_blobs
        if (len(source["files"]) != total
                or len(blobs) != distinct or size_sum != byte_total
                or source["selected_file_occurrences"] != total
                or source["distinct_git_blobs"] != distinct
                or source["selected_bytes"] != byte_total
                or source["matching_smoker1_file_occurrences"] !=
                   (matching if identifier == "eignar17-arcade" else None)):
            raise ValueError("Source counts or limited byte overlap do not reconcile")
        count += total
    if (doc["source_file_occurrences"] != count
            or doc["distinct_git_blobs_in_source_union"] != len(union)):
        raise ValueError("Scout totals do not reconcile")
    return True

def live_report(doc):
    import copy
    out = copy.deepcopy(doc)
    for source, spec in zip(out["sources"], SOURCES):
        ident, repo, commit, _, _, _ = spec
        url = f"https://api.github.com/repos/{repo}/git/trees/{commit}?recursive=1"
        headers = {"Accept": "application/vnd.github+json",
                   "User-Agent": "Cheatarium-metadata-only-bulk-scout"}
        if os.environ.get("GITHUB_TOKEN"):
            headers["Authorization"] = "Bearer " + os.environ["GITHUB_TOKEN"]
        with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=60) as response:
            tree = json.load(response)
        if tree.get("truncated"):
            raise ValueError("Pinned Git tree was truncated: " + ident)
        files = sorted(([x["path"], x["sha"], x["size"]] for x in tree["tree"]
                        if x["type"] == "blob" and selected(ident, x["path"])),
                       key=lambda x: x[0])
        source["source_tree_sha"] = tree["sha"]
        source["files"] = files
        source["selected_file_occurrences"] = len(files)
        source["distinct_git_blobs"] = len({x[1] for x in files})
        source["selected_bytes"] = sum(x[2] for x in files)
        if ident == "eignar17-arcade":
            baseline = json.loads((ROOT / "sources/external-bulk/smoker1-fba-official-ini.json").read_text("utf-8"))
            known = {f[1] for f in baseline["files"]}
            source["matching_smoker1_file_occurrences"] = sum(x[1] in known for x in files)
    out["source_file_occurrences"] = sum(x["selected_file_occurrences"] for x in out["sources"])
    out["distinct_git_blobs_in_source_union"] = len({x[1] for s in out["sources"] for x in s["files"]})
    verify(out)
    return out

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--offline-check", action="store_true")
    group.add_argument("--verify-upstream", action="store_true")
    group.add_argument("--write", action="store_true")
    args = parser.parse_args()
    document = json.loads(FILE.read_text("utf-8"))
    verify(document)
    if args.verify_upstream or args.write:
        regenerated = live_report(document)
        if args.verify_upstream and regenerated != document:
            raise ValueError("Pinned Git tree metadata has drifted")
        if args.write:
            FILE.write_text(json.dumps(regenerated, ensure_ascii=False, indent=2) + "\n", "utf-8")
    print("OK: 691 rights-unresolved source occurrences across four pinned trees")

if __name__ == "__main__":
    main()
