#!/usr/bin/env python3
"""Independently audit SNES '+' semantics and evidenced source-code partitions.

This never reads ROMs, runs emulators, or treats generic '+' as simultaneous
execution. Source registry may contain documented revision-alternative layouts.
"""
import argparse
import gzip
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REVISION = "revision-alternatives"
UNRESOLVED = "unresolved"


def audit(root):
    original = json.loads((root / "interpretations/v1/snes.json").read_text())
    copied = json.loads((root / "generated/v1/interpretations/snes.json").read_text())
    if original != copied:
        raise ValueError("Published SNES compositions differ from reviewed registry")
    if (original.get("schema_version") != 1 or original.get("platform") != "snes"
            or original.get("interpretation") != "evidenced-source-code-layout-not-ROM-compatibility"
            or not isinstance(original.get("records"), list)):
        raise ValueError("Invalid SNES composition registry header")
    reviewed = {}
    for evidence in original["records"]:
        key = (evidence["source_record_id"], evidence["source_ordinal"])
        if key in reviewed:
            raise ValueError(f"Duplicate source/ordinal composition evidence: {key}")
        if (evidence["relation"] != REVISION or evidence["rom_match_verified"] is not False
                or evidence["simultaneous_execution_confirmed"] is not False):
            raise ValueError("False or unsupported interpretation claim")
        alternatives = evidence["alternatives"]
        if (len(alternatives) < 2
                or any(not group or any(not isinstance(c, str) or not c or c.strip() != c for c in group)
                       for group in alternatives)):
            raise ValueError("Malformed revision-alternative partition")
        original_parts = [x.strip() for x in evidence["raw_code"].split("+")]
        if (len(original_parts) < 2
                or [x for group in alternatives for x in group] != original_parts):
            raise ValueError("Alternatives must partition the unchanged raw source code")
        refs = evidence["evidence"]
        if not isinstance(refs, list) or not refs or any(
            not isinstance(ref, dict)
            or not isinstance(ref.get("url"), str) or not ref["url"].startswith("https://")
            or not isinstance(ref.get("reference"), str) or not ref["reference"].strip()
            or not isinstance(ref.get("source_revision"), str) or not ref["source_revision"].strip()
            for ref in refs
        ):
            raise ValueError("Unattributed evidence for source composition")
        reviewed[key] = evidence
    with gzip.open(root / "generated/v1/snes.json.gz", "rt") as stream:
        bundle = json.load(stream)
    if bundle["schema_version"] != 1 or bundle["platform"] != "snes":
        raise ValueError("Invalid SNES bundle")
    counts = {"reviewed_revision_alternatives": 0, "unresolved_plus_groups": 0,
              "unknown_plus_groups_with_decoded_components": 0}
    seen = set()
    for record in bundle["records"]:
        source_id = record["id"]
        for entry in record["codes"]:
            raw = entry.get("code") or ""
            composition = entry.get("composition")
            key = (source_id, entry["ordinal"])
            if entry["role"] != "code" or "+" not in raw:
                if composition is not None:
                    raise ValueError(f"Unsupported source composition on {key}")
                continue
            expected = reviewed.get(key)
            if expected is None:
                if composition != {
                    "relation": UNRESOLVED, "alternatives": [],
                    "evidence": [], "rom_match_verified": False,
                    "simultaneous_execution_confirmed": False
                }:
                    raise ValueError(f"Unreviewed '+' source has fabricated semantics: {key}")
                counts["unresolved_plus_groups"] += 1
                if entry.get("snes_decode"):
                    counts["unknown_plus_groups_with_decoded_components"] += 1
                continue
            seen.add(key)
            if raw != expected["raw_code"] or record["provenance"]["git_blob_sha"] != expected["source_git_blob_sha"]:
                raise ValueError(f"Reviewed raw source code/blob changed: {key}")
            if composition != {
                "relation": expected["relation"],
                "alternatives": expected["alternatives"],
                "evidence": expected["evidence"],
                "rom_match_verified": False,
                "simultaneous_execution_confirmed": False
            }:
                raise ValueError(f"Reviewed composition is stale or modified: {key}")
            if entry.get("snes_decode") is not None:
                raise ValueError(f"Version alternatives emitted as simultaneous decoded writes: {key}")
            counts["reviewed_revision_alternatives"] += 1
    if seen != set(reviewed):
        raise ValueError(f"Missing reviewed source+ordinal interpretations: {set(reviewed)-seen}")
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    counts = audit(args.root)
    print("OK: source-code composition evidence:", json.dumps(counts, sort_keys=True))


if __name__ == "__main__":
    main()
