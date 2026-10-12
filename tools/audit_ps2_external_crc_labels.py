#!/usr/bin/env python3
"""Audit exact PS2 PNACH *filename* CRC and serial labels across pinned references.

No cheat contents are used. Matching an eight-hex-digit CRC or a SERIAL_CRC
filename is source-identification evidence ONLY: it is not a verified game,
release, cheat effect, executable or compatible ROM.
"""
import argparse
import itertools
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

from external_bulk import ROOT, load

REL = Path("generated/v1/reports/external-ps2-crc-labels.json")
FORMAT = "cheatarium-external-ps2-native-filename-labels-v1"
SERIAL_CRC = re.compile(r"(?P<serial>[A-Z]{4}-[0-9]{5})_(?P<crc>[0-9A-Fa-f]{8})\Z")
CRC_ONLY = re.compile(r"(?P<crc>[0-9A-Fa-f]{8})\Z")
CRC_PREFIX = re.compile(r"(?P<crc>[0-9A-Fa-f]{8}) [-].+\Z")


def parse_pnach_filename(path):
    name = path.rsplit("/", 1)[-1]
    if not name.endswith(".pnach"):
        return None
    stem = name[:-6]
    if match := SERIAL_CRC.fullmatch(stem):
        return {"format": "serial_crc", "crc": match["crc"].upper(),
                "serial": match["serial"]}
    if match := CRC_ONLY.fullmatch(stem):
        return {"format": "crc_only", "crc": match["crc"].upper(),
                "serial": None}
    if match := CRC_PREFIX.fullmatch(stem):
        return {"format": "crc_prefix_title", "crc": match["crc"].upper(),
                "serial": None}
    return None


def compile_report(sources):
    counts_by_source = {}
    grouped_crc = defaultdict(set)
    per_source_crc = {}
    per_source_pair = {}
    unresolved_total = 0
    recognized_total = 0
    total_paths = 0
    for source in sources:
        if source["platform"] != "ps2":
            raise ValueError("PS2 filename audit was supplied a different platform")
        sid = source["id"]
        if sid in counts_by_source:
            raise ValueError("Duplicate PS2 source ID")
        counter = Counter()
        crc_set = set()
        serial_pair_set = set()
        malformed = []
        for path, _sha, _size in source["files"]:
            total_paths += 1
            label = parse_pnach_filename(path)
            if label is None:
                counter["unrecognized"] += 1
                unresolved_total += 1
                if len(malformed) < 12:
                    malformed.append(path)
                continue
            counter[label["format"]] += 1
            recognized_total += 1
            crc = label["crc"]
            crc_set.add(crc)
            grouped_crc[crc].add(sid)
            if label["serial"] is not None:
                serial_pair_set.add((label["serial"], crc))
        per_source_crc[sid] = crc_set
        per_source_pair[sid] = serial_pair_set
        counts_by_source[sid] = {
            "source_id": sid,
            "native_pnach_file_occurrences": len(source["files"]),
            "recognized_strict_crc_labels": sum(
                counter[k] for k in ("serial_crc", "crc_only", "crc_prefix_title")),
            "recognizable_serial_crc_filenames": counter["serial_crc"],
            "crc_only_filenames": counter["crc_only"],
            "crc_prefixed_title_filenames": counter["crc_prefix_title"],
            "unrecognized_or_malformed_filename_labels": counter["unrecognized"],
            "distinct_crc_label_strings": len(crc_set),
            "distinct_strict_serial_crc_label_pairs": len(serial_pair_set),
            "unrecognized_samples": malformed,
        }
    if sum(x["native_pnach_file_occurrences"] for x in counts_by_source.values()) != total_paths:
        raise ValueError("PS2 source inventory mismatch")
    source_ids = sorted(counts_by_source)
    shared = sorted(k for k, owners in grouped_crc.items() if len(owners) > 1)
    pair_rows = []
    for left, right in itertools.combinations(source_ids, 2):
        a = per_source_crc[left]
        b = per_source_crc[right]
        ap = per_source_pair[left]
        bp = per_source_pair[right]
        pair_rows.append({
            "source_a": left,
            "source_b": right,
            "shared_crc_label_strings": len(a & b),
            "crc_labels_only_a": len(a - b),
            "crc_labels_only_b": len(b - a),
            "shared_strict_serial_crc_label_pairs": len(ap & bp),
            "serial_crc_label_pairs_only_a": len(ap - bp),
            "serial_crc_label_pairs_only_b": len(bp - ap),
        })
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "strict-case-normalized-pnach-filename-crc-and-serial-labels-only",
        "platform": "ps2",
        "link_only": True,
        "cheat_or_patch_payloads_included": False,
        "independently_verified_games_or_cheat_effects": False,
        "filename_labels_are_verified_game_or_rom_build_ids": False,
        "crc_collisions_or_aliases_excluded": False,
        "source_catalogs": len(sources),
        "source_native_pnach_files": total_paths,
        "recognized_crc_filename_occurrences": recognized_total,
        "unrecognized_crc_filename_occurrences": unresolved_total,
        "distinct_crc_label_strings": len(grouped_crc),
        "crc_labels_in_two_or_more_sources": len(shared),
        "source_rows": [counts_by_source[id] for id in source_ids],
        "pairwise_source_label_overlap": pair_rows,
        "shared_crc_label_examples": [
            {"crc": k, "sources": sorted(grouped_crc[k])}
            for k in shared[:40]
        ],
    }


def build(root=ROOT):
    sources = [x for x in load(root) if x["platform"] == "ps2"]
    if len(sources) != 4:
        raise ValueError("Four pinned PS2 external native-PNACH catalogs expected")
    report = compile_report(sources)
    if report["source_native_pnach_files"] != 14404:
        raise ValueError("Pinned PS2 native-PNACH file count differs")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    report = build()
    output = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    target = ROOT / REL
    if args.write:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(output, encoding="utf-8")
    elif not target.is_file() or target.read_text("utf-8") != output:
        raise SystemExit("External PS2 filename identity audit is stale or modified")
    print(f"OK: {report['source_native_pnach_files']} PNACH references, "
          f"{report['recognized_crc_filename_occurrences']} strict CRC label occurrences, "
          f"{report['crc_labels_in_two_or_more_sources']} cross-source CRC label strings")


if __name__ == "__main__":
    main()
