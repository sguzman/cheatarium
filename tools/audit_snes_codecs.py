#!/usr/bin/env python3
"""Audit the strictly decoded SNES source-code coverage, without guessing formats."""
import argparse
import gzip
import json
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPLICIT = {"game-genie", "action-replay"}


def audit(index_path):
    with gzip.open(index_path, "rt", encoding="utf-8") as file:
        bundle = json.load(file)
    if bundle["schema_version"] != 1 or bundle["platform"] != "snes":
        raise ValueError("Expected Cheatarium SNES v1 source bundle")
    counts = Counter()
    samples = []
    for source in bundle["records"]:
        format_hint = source.get("format_hint")
        for entry in source["codes"]:
            if entry["role"] == "memory-entry":
                counts["native_memory_entries"] += 1
                continue
            if entry["role"] == "section-heading":
                counts["section_headings"] += 1
                continue
            if entry["role"] != "code":
                raise ValueError("Unexpected source role")
            counts["encoded_code_fields"] += 1
            parsed = entry.get("snes_decode")
            if parsed:
                if format_hint not in EXPLICIT or parsed.get("format") != format_hint:
                    raise ValueError("Decoded entry is missing an explicit matching format")
                counts["strictly_decoded"] += 1
                continue
            if format_hint in EXPLICIT:
                counts["explicit_format_not_decodable"] += 1
                if len(samples) < 12:
                    samples.append({
                        "source": source["id"],
                        "ordinal": entry["ordinal"],
                        "format_hint": format_hint,
                        "source_code": entry.get("code"),
                    })
            elif format_hint:
                counts["other_device_format"] += 1
            else:
                counts["format_not_identified"] += 1
    subtotal = (counts["strictly_decoded"] + counts["explicit_format_not_decodable"]
                + counts["other_device_format"] + counts["format_not_identified"])
    if subtotal != counts["encoded_code_fields"]:
        raise ValueError("Codec coverage buckets do not account for every source code")
    return {
        "schema_version": 1,
        "scope": "snes",
        "status": "source_decryption_only_not_rom_verified",
        "source_files": len(bundle["records"]),
        "counts": dict(sorted(counts.items())),
        "sample_undecoded_explicit_codes": samples,
        "remainder_policy": "Do not invent missing format hints, placeholder values, or ROM compatibility",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--index", type=Path, default=ROOT / "generated/v1/snes.json.gz")
    parser.add_argument("--json", action="store_true", help="Print deterministic JSON audit")
    args = parser.parse_args()
    report = audit(args.index)
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        c = report["counts"]
        print("SNES device-code audit (source occurrences, NOT verified cheats)")
        for label, n in c.items():
            print(f"  {label}: {n:,}")
        print("Unrecognized formats and placeholders remain uninterpreted.")


if __name__ == "__main__":
    main()
