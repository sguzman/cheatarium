#!/usr/bin/env python3
"""Compare pinned Switch inventories by original title/build IDs, not guessed games.

Shared identifiers do not prove code equivalence or gameplay compatibility.
"""
import argparse
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REPORT = "generated/v1/reports/switch-bulk-source-coverage.json"
HEX16 = re.compile(r"^[0-9a-fA-F]{16}$")
SHARKIVE = "sharkive"
NX = "bad1dea-nxcheats-switch"


def source_pairs(manifest, source):
    paths = [row["upstream_path"] for row in manifest["files"]]
    native_count = 0
    uncomparable = []
    title_ids = set()
    pair_counts = Counter()
    for source_path in paths:
        parts = source_path.split("/")
        filename = parts[-1]
        if source == SHARKIVE:
            if len(parts) != 3 or parts[0] != "switch" or not filename.endswith(".txt"):
                continue
            native_count += 1
            title = parts[1]
        else:
            build = filename.removesuffix(".txt")
            if (not filename.endswith(".txt") or not HEX16.fullmatch(build)
                    or "cheats" not in parts[:-1]):
                continue
            native_count += 1
            title = next((part.split(" ", 1)[0] for part in parts[:-1]
                          if HEX16.fullmatch(part.split(" ", 1)[0])), "")
            if not title:
                raise ValueError(f"NXCheats native file missing title ID: {source_path}")
        title_ids.add(title.lower())
        build = filename.removesuffix(".txt")
        if HEX16.fullmatch(title) and HEX16.fullmatch(build):
            pair_counts[(title.lower(), build.lower())] += 1
        else:
            uncomparable.append(source_path)
    if source == NX:
        expected = manifest.get("source_native_cheat_txt_files")
        if not isinstance(expected, int) or native_count != expected:
            raise ValueError(f"NXCheats native inventory mismatch: {native_count} != {expected}")
    return {
        "original_manifest_files_including_noncheats": len(paths),
        "native_cheat_source_files": native_count,
        "distinct_title_id_labels": len(title_ids),
        "distinct_title_build_id_pairs": len(pair_counts),
        "duplicate_source_paths_for_same_pair": sum(n - 1 for n in pair_counts.values()),
        "noncomparable_native_paths": sorted(uncomparable),
    }, title_ids, set(pair_counts)


def build_report(root):
    originals = {}
    for source in (SHARKIVE, NX):
        originals[source] = json.loads(
            (root / "sources" / (source + ".json")).read_text("utf-8"))
        if originals[source].get("id") != source:
            raise ValueError(f"Mismatched source ID: {source}")
    shark, shark_titles, shark_pairs = source_pairs(originals[SHARKIVE], SHARKIVE)
    nx, nx_titles, nx_pairs = source_pairs(originals[NX], NX)
    return {
        "schema_version": 1,
        "comparison_basis": "source_path_title_id_and_16_hex_build_id_only_unverified",
        "sources": {
            SHARKIVE: {**shark, "revision": originals[SHARKIVE]["snapshot_commit"]},
            NX: {**nx, "revision": originals[NX]["snapshot_commit"]},
        },
        "title_id_labels": {
            "shared": len(shark_titles & nx_titles),
            "only_sharkive": len(shark_titles - nx_titles),
            "only_nxcheats": len(nx_titles - shark_titles),
            "combined_distinct": len(shark_titles | nx_titles),
        },
        "title_build_id_pairs": {
            "shared": len(shark_pairs & nx_pairs),
            "only_sharkive": len(shark_pairs - nx_pairs),
            "only_nxcheats": len(nx_pairs - shark_pairs),
            "combined_distinct": len(shark_pairs | nx_pairs),
        },
        "caveats": [
            "Source title IDs are author-supplied labels, not independently confirmed game identity.",
            "Title ID plus build ID overlap does not prove code equivalence, operation, or compatibility.",
            "Repeated paths and historic one-effect files are not distinct games or tested effects.",
            "Unknown and nonstandard build-ID paths are not forced into invented matches.",
            "Independent historical author redistribution rights remain source-specific.",
        ],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    report = build_report(args.root)
    payload = json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True) + "\n"
    path = args.root / REPORT
    if args.check:
        if not path.exists() or path.read_text("utf-8") != payload:
            raise SystemExit("Switch bulk-coverage report missing or stale")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(payload, "utf-8")
    print(json.dumps({
        "status": "valid" if args.check else "written",
        "title_id_labels": report["title_id_labels"],
        "title_build_id_pairs": report["title_build_id_pairs"],
    }))


if __name__ == "__main__":
    main()
