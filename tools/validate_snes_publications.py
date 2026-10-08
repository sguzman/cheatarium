#!/usr/bin/env python3
"""Validate external SNES multi-part publication witnesses against pinned source.

Publication evidence is not gameplay observation or permission to combine,
activate or choose codes for any particular cartridge revision.
"""
import argparse
import gzip
import json
import re
from pathlib import Path

from audit_snes_publication_archive import audit_original_archives

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path("interpretations/v1/snes-published-groups.json")
PUBLISHED = Path("generated/v1/interpretations/snes-published-groups.json")
CLAIM_TYPE = "externally-published-multi-part-source-code-text"


def validate_registry(registry, bundle):
    if (registry.get("schema_version") != 1
            or registry.get("platform") != "snes"
            or registry.get("claim_type") != CLAIM_TYPE
            or not isinstance(registry.get("evidence_limit"), str)
            or not registry["evidence_limit"].strip()
            or not isinstance(registry.get("records"), list)
            or bundle.get("schema_version") != 1
            or bundle.get("platform") != "snes"):
        raise ValueError("Invalid SNES publication witness schema")
    sources = {r["id"]: r for r in bundle["records"]}
    seen = set()
    for report in registry["records"]:
        key = (report["source_record_id"], report["source_ordinal"])
        if key in seen:
            raise ValueError(f"Duplicate historical publication witness: {key}")
        seen.add(key)
        raw = report["raw_code"]
        segments = report.get("published_text_segments")
        if (not isinstance(raw, str) or "+" not in raw
                or not isinstance(segments, list) or len(segments) < 2
                or any(not isinstance(s, str) or not s or s != s.strip()
                       for s in segments)
                or segments != [s.strip() for s in raw.split("+")]
                or not isinstance(report.get("published_effect_description"), str)
                or not report["published_effect_description"].strip()
                or report.get("execution_observed") is not False
                or report.get("rom_match_verified") is not False
                or report.get("safe_to_auto_apply") is not False):
            raise ValueError(f"Invalid multi-part publication record: {key}")
        pub = report.get("publication")
        if (not isinstance(pub, dict)
                or not isinstance(pub.get("url"), str)
                or not pub["url"].startswith("https://")
                or any(not isinstance(pub.get(k), str) or not pub[k].strip()
                       for k in ("reference", "source_revision"))):
            raise ValueError(f"Missing external publication attribution: {key}")
        record = sources.get(report["source_record_id"])
        if record is None:
            raise ValueError(f"Unknown original source record: {key}")
        candidate = record["candidate_game_key"] or f'unresolved:{record["id"]}'
        if candidate != report["candidate_game_key"]:
            raise ValueError(f"Candidate game-key mismatch: {key}")
        sha = record["provenance"]["git_blob_sha"]
        if (not re.fullmatch(r"[0-9a-f]{40}", report["source_git_blob_sha"])
                or sha != report["source_git_blob_sha"]):
            raise ValueError(f"Original source Git blob differs from witness: {key}")
        matches = [c for c in record["codes"] if c["ordinal"] == report["source_ordinal"]]
        if len(matches) != 1 or matches[0].get("role") != "code":
            raise ValueError(f"Missing exact original source ordinal: {key}")
        cheat = matches[0]
        if cheat.get("code") != raw or (cheat.get("composition") or {}).get(
                "relation") != "unresolved":
            raise ValueError(f"Witness does not describe unchanged unresolved source: {key}")
    return {"historical_publication_witnesses": len(seen),
            "empirically_tested_combinations": 0}


def audit(root=ROOT, *, write=False):
    source = root / SOURCE
    published = root / PUBLISHED
    raw_bytes = source.read_bytes()
    registry = json.loads(raw_bytes)
    with gzip.open(root / "generated/v1/snes.json.gz", "rt", encoding="utf-8") as stream:
        bundle = json.load(stream)
    stats = validate_registry(registry, bundle)
    stats.update(audit_original_archives(root, registry, bundle))
    if write:
        published.parent.mkdir(parents=True, exist_ok=True)
        published.write_bytes(raw_bytes)
    elif published.read_bytes() != raw_bytes:
        raise ValueError("Published SNES source witness registry is stale or modified")
    return stats


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    print("OK: SNES historical publication witnesses:",
          json.dumps(audit(args.root, write=args.write), sort_keys=True))


if __name__ == "__main__":
    main()
