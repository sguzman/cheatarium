#!/usr/bin/env python3
"""Audit the SNES decoded-code coverage without inferring game compatibility."""
import argparse
import gzip
import json
import re
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXPLICIT = {"game-genie", "action-replay"}
SYNTAX_ONLY = {"game-genie", "raw-snes-address-value"}
GG = re.compile(r"[DF4709156BC8A23E]{4}-[DF4709156BC8A23E]{4}", re.I)
RAW = re.compile(r"[0-9a-f]{8}", re.I)


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
                if entry.get("snes_decode"):
                    raise ValueError("Native memory entry cannot become a device code")
                counts["native_memory_entries"] += 1
                continue
            if entry["role"] == "section-heading":
                if entry.get("snes_decode"):
                    raise ValueError("Section heading cannot become a device code")
                counts["section_headings"] += 1
                continue
            if entry["role"] != "code":
                raise ValueError("Unexpected source role")
            counts["encoded_code_fields"] += 1
            parsed = entry.get("snes_decode")
            if parsed:
                if parsed.get("compatibility") != "unverified-cartridge-build":
                    raise ValueError("Never promote inferred text to verified ROM compatibility")
                if parsed.get("address_space") != "snes-cpu-bus-24-bit":
                    raise ValueError("Unsupported SNES CPU bus address-space label")
                basis = parsed.get("interpretation_basis")
                code_text = entry.get("code", "")
                parts = [part.strip() for part in code_text.split("+")]
                if len(parts) != len(parsed.get("writes", [])):
                    raise ValueError("Decoder lost one or more compound parts")
                if basis == "declared-file-format":
                    if format_hint not in EXPLICIT or parsed.get("format") != format_hint:
                        raise ValueError("Decoded format differs from declared filename")
                    counts["decoded_declared_format"] += 1
                elif basis == "code-syntax":
                    if format_hint is not None or parsed.get("format") not in SYNTAX_ONLY:
                        raise ValueError("Syntax inference overrides nonempty source device hint")
                    pattern = GG if parsed["format"] == "game-genie" else RAW
                    if not parts or not all(pattern.fullmatch(part) for part in parts):
                        raise ValueError("Inferred SNES format is unsupported by its code syntax")
                    if parsed["format"] == "game-genie":
                        counts["anonymous_game_genie_codes"] += 1
                    else:
                        counts["anonymous_hex_address_candidates"] += 1
                    counts["decoded_from_code_syntax"] += 1
                else:
                    raise ValueError("Every SNES decoding must state its evidence basis")
                for write in parsed["writes"]:
                    address, value = write.get("address_hex", ""), write.get("value_hex", "")
                    if not re.fullmatch(r"[0-9A-F]{6}", address) or not re.fullmatch(r"[0-9A-F]{2}", value):
                        raise ValueError("Malformed decoded SNES address or byte")
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

    subtotal = sum(counts[k] for k in (
        "strictly_decoded",
        "explicit_format_not_decodable",
        "other_device_format",
        "format_not_identified",
    ))
    if subtotal != counts["encoded_code_fields"]:
        raise ValueError("Coverage buckets do not account for every source code")
    if counts["decoded_declared_format"] + counts["decoded_from_code_syntax"] != counts["strictly_decoded"]:
        raise ValueError("SNES code origin audit disagrees with decoded total")
    return {
        "schema_version": 1,
        "scope": "snes",
        "status": "syntax_interpretation_only_not_rom_verified",
        "source_files": len(bundle["records"]),
        "counts": dict(sorted(counts.items())),
        "sample_undecoded_explicit_codes": samples,
        "remainder_policy": "Never infer ROM compatibility or change original source codes",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--index", type=Path, default=ROOT / "generated/v1/snes.json.gz")
    parser.add_argument("--out", type=Path, help="Write deterministic coverage report as an artifact")
    parser.add_argument("--json", action="store_true", help="Print deterministic JSON audit")
    args = parser.parse_args()
    report = audit(args.index)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print("SNES source-code coverage audit (NOT verified cheats)")
        for label, n in report["counts"].items():
            print(f"  {label}: {n:,}")
        print("Unrecognized formats and placeholders remain uninterpreted.")


if __name__ == "__main__":
    main()
