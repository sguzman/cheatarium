#!/usr/bin/env python3
"""Build/check a deterministic v1 artifact manifest and cross-file invariants.

Only operates on generated files. No network, ROMs, emulator calls or upstream edits.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    data = path.read_bytes()
    return {"sha256": hashlib.sha256(data).hexdigest(), "size_bytes": len(data)}


def safe_path(root, relative):
    posix = PurePosixPath(relative)
    if posix.is_absolute() or any(part in ("", ".", "..") for part in posix.parts):
        raise ValueError(f"Unsafe artifact path {relative!r}")
    return root.joinpath(*posix.parts)


def checked_json(path):
    if path.name.endswith(".gz"):
        with gzip.open(path, "rt", encoding="utf-8") as handle:
            return json.load(handle)
    return json.loads(path.read_text(encoding="utf-8"))


def build_manifest(root):
    catalog = checked_json(root / "catalog.json")
    if catalog.get("schema_version") != 1 or catalog.get("format") != "cheatarium-index-v1":
        raise ValueError("Unsupported source catalog schema")
    entries = catalog.get("bundles")
    if not isinstance(entries, list) or not entries:
        raise ValueError("No per-console catalog entries")

    manifest_files = []
    named_paths = set()
    source_files = 0
    game_groups = 0
    decoded_snes_entries = 0
    for rel in ["catalog.json", "reports/snes-codec-coverage.json"]:
        path = safe_path(root, rel)
        manifest_files.append({"path": rel, **digest(path)})
        named_paths.add(rel)
    coverage = checked_json(safe_path(root, "reports/snes-codec-coverage.json"))
    if coverage.get("schema_version") != 1 or coverage.get("scope") != "snes":
        raise ValueError("Invalid SNES decoder audit report")

    for entry in entries:
        platform = entry["platform"]
        if not platform or not all(c.islower() or c.isdigit() or c == "-" for c in platform):
            raise ValueError(f"Invalid platform id {platform!r}")
        source_rel = entry["artifact"]
        game_rel = entry["game_index_artifact"]
        if source_rel != f"{platform}.json.gz" or game_rel != f"games/{platform}.json.gz":
            raise ValueError(f"Unexpected artifact path for {platform}")
        identity_rel = entry.get("identity_artifact")
        artifacts = [source_rel, game_rel]
        if identity_rel is not None:
            if identity_rel != f"identities/{platform}.json":
                raise ValueError(f"Unexpected ROM identity artifact path for {platform}")
            identity = checked_json(safe_path(root, identity_rel))
            if (identity.get("schema_version") != 1
                    or identity.get("platform") != platform
                    or identity.get("hash_scope") != "sha256-entire-file-unaltered"
                    or not isinstance(identity.get("records"), list)):
                raise ValueError(f"Invalid ROM identity registry: {platform}")
            for record in identity["records"]:
                digest_hex = record.get("sha256", "")
                if (len(digest_hex) != 64
                        or not all(c in "0123456789abcdef" for c in digest_hex)
                        or not isinstance(record.get("byte_length"), int)
                        or record["byte_length"] <= 0
                        or not all(record.get(field) for field in ("game_id", "title", "edition_id"))
                        or not isinstance(record.get("evidence"), list)
                        or not record["evidence"]):
                    raise ValueError(f"Bad identity claim in {platform}")
                for evidence in record["evidence"]:
                    if (not evidence.get("url", "").startswith("https://")
                            or not evidence.get("reference")
                            or not evidence.get("source_revision")
                            or evidence.get("review_state") not in ("candidate", "reviewed")):
                        raise ValueError(f"Invalid ROM fingerprint evidence in {platform}")
            artifacts.append(identity_rel)
        for rel in artifacts:
            if rel in named_paths:
                raise ValueError(f"Duplicate artifact path {rel}")
            manifest_files.append({"path": rel, **digest(safe_path(root, rel))})
            named_paths.add(rel)

        bundle = checked_json(safe_path(root, source_rel))
        games = checked_json(safe_path(root, game_rel))
        if bundle["schema_version"] != 1 or bundle["platform"] != platform:
            raise ValueError(f"Bad source bundle header: {platform}")
        if games["schema_version"] != 1 or games["platform"] != platform:
            raise ValueError(f"Bad game index header: {platform}")
        records = bundle["records"]
        groups = games["candidates"]
        if len(records) != entry["source_files"] or len(groups) != entry["game_candidate_groups"]:
            raise ValueError(f"Incorrect record or group count: {platform}")

        record_ids = [r["id"] for r in records]
        if len(record_ids) != len(set(record_ids)):
            raise ValueError(f"Duplicate source record IDs: {platform}")
        all_links = []
        group_keys = set()
        game_code_fields = 0
        game_memory_fields = 0
        for group in groups:
            if group["key"] in group_keys:
                raise ValueError(f"Duplicate game candidate key: {platform} {group['key']}")
            group_keys.add(group["key"])
            if group["identity_confidence"] != "filename_candidate_only":
                raise ValueError(f"Unreviewed game identity promotion: {platform}")
            all_links.extend(group["source_record_ids"])
            game_code_fields += group["code_fields"]
            game_memory_fields += group["native_memory_entries"]
        if sorted(all_links) != sorted(record_ids):
            raise ValueError(f"Source occurrences missing/duplicated in game groups: {platform}")
        encoded = sum(c["role"] == "code" for r in records for c in r["codes"])
        native = sum(c["role"] == "memory-entry" for r in records for c in r["codes"])
        decoded = 0
        for record in records:
            for code in record["codes"]:
                interpretation = code.get("snes_decode")
                if interpretation is None:
                    continue
                if platform != "snes" or code["role"] != "code":
                    raise ValueError(f"Unexpected SNES decoding field: {platform}")
                source_format = record.get("format_hint")
                decoded_format = interpretation.get("format")
                evidence = interpretation.get("interpretation_basis")
                if evidence == "declared-file-format":
                    if source_format not in ("game-genie", "action-replay") or decoded_format != source_format:
                        raise ValueError("Decoder contradicted the declared file format")
                elif evidence == "code-syntax":
                    if source_format is not None or decoded_format not in (
                        "game-genie", "raw-snes-address-value"
                    ):
                        raise ValueError("Code syntax cannot override a declared file format")
                else:
                    raise ValueError("Missing or invalid SNES decoder evidence basis")
                if interpretation.get("compatibility") != "unverified-cartridge-build":
                    raise ValueError("Decoded code incorrectly claims ROM compatibility")
                if interpretation.get("address_space") != "snes-cpu-bus-24-bit":
                    raise ValueError("Unsupported decoded address space")
                writes = interpretation.get("writes", [])
                if not writes:
                    raise ValueError("Decoded compound code contains no entries")
                for patch in writes:
                    addr = patch.get("address_hex", "")
                    value = patch.get("value_hex", "")
                    if len(addr) != 6 or len(value) != 2 or not all(c in "0123456789ABCDEF" for c in addr + value):
                        raise ValueError("Malformed SNES decoded address/value")
                decoded += 1
        if decoded != entry.get("decoded_snes_code_fields", 0):
            raise ValueError(f"Incorrect SNES decoder statistics: {platform}")
        if platform == "snes" and (
            coverage.get("counts", {}).get("strictly_decoded") != decoded
            or coverage.get("source_files") != len(records)
        ):
            raise ValueError("SNES coverage report disagrees with exported source data")
        decoded_snes_entries += decoded
        if (encoded, native) != (entry["code_fields"], entry["native_memory_entries"]):
            raise ValueError(f"Catalog/source counts differ: {platform}")
        if (game_code_fields, game_memory_fields) != (encoded, native):
            raise ValueError(f"Game grouping/source counts differ: {platform}")
        source_files += len(records)
        game_groups += len(groups)

    return {
        "schema_version": 1,
        "format": "cheatarium-distribution-v1",
        "compatibility": "filename candidates only; never auto-enable cheat codes",
        "console_bundles": len(entries),
        "source_files": source_files,
        "game_candidate_groups": game_groups,
        "decoded_snes_code_fields": decoded_snes_entries,
        "files": sorted(manifest_files, key=lambda item: item["path"]),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT / "generated/v1")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Create deterministic SHA-256 distribution manifest")
    mode.add_argument("--check", action="store_true", help="Verify all files against the existing manifest")
    args = parser.parse_args()
    actual = build_manifest(args.root)
    path = args.root / "distribution.json"
    if args.write:
        path.write_text(json.dumps(actual, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"Wrote {path}: {actual['console_bundles']} consoles, {len(actual['files'])} artifacts")
    else:
        expected = checked_json(path)
        if actual != expected:
            raise SystemExit("Distribution manifest differs from generated files; rebuild required")
        print(f"OK: {actual['console_bundles']} consoles, {actual['source_files']} source records, "
              f"{actual['game_candidate_groups']} game candidate groups; all SHA-256 hashes agree")


if __name__ == "__main__":
    main()
