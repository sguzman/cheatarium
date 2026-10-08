#!/usr/bin/env python3
"""Build/check a deterministic v1 artifact manifest and cross-file invariants.

Only operates on generated files. No network, ROMs, emulator calls or upstream edits.
"""
import argparse
import gzip
import hashlib
import json
import re
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
    repeated_code_groups = 0
    lexical_tag_matches = 0
    from validate_snes_compositions import audit as audit_compositions
    compositions = audit_compositions(ROOT)
    from validate_snes_publications import audit as audit_publications
    publication_witnesses = audit_publications(ROOT)
    if publication_witnesses["empirically_tested_combinations"] != 0:
        raise ValueError("Publication witness registry cannot assert empirical execution")
    if publication_witnesses["publication_witness_ordinals_checked"] != publication_witnesses["historical_publication_witnesses"]:
        raise ValueError("Original archive audit did not cover every publication witness")
    from build_snes_review_queue import build as rebuild_composition_queue
    queue_rel = "reports/snes-composition-review-queue.json"
    composition_queue = checked_json(safe_path(root, queue_rel))
    if composition_queue != rebuild_composition_queue(ROOT):
        raise ValueError("Stale or tampered source composition review queue")
    witness_count = publication_witnesses["historical_publication_witnesses"]
    if (composition_queue.get("total_historical_publication_witnesses") != witness_count
            or composition_queue.get("total_without_publication_witness") !=
            composition_queue["total_unresolved_source_occurrences"] - witness_count):
        raise ValueError("SNES publication witness counts disagree with research queue")
    composition_rel = "interpretations/snes.json"
    taxonomy_rel = "taxonomy/effects-v1.json"
    taxonomy = checked_json(safe_path(root, taxonomy_rel))
    if (taxonomy.get("schema_version") != 1
            or taxonomy.get("id") != "cheatarium-effect-signals-en-v1"
            or taxonomy.get("language") != "en"
            or taxonomy.get("method") != "ascii-token-phrase"
            or not isinstance(taxonomy.get("categories"), list)
            or not taxonomy["categories"]):
        raise ValueError("Unsupported effect-signal taxonomy")
    categories = taxonomy["categories"]
    ids = set()
    for category in categories:
        cid = category.get("id", "")
        phrases = category.get("phrases", [])
        if (not isinstance(cid, str) or not re.fullmatch(r"[a-z-]+", cid)
                or cid in ids or not category.get("label")
                or not isinstance(phrases, list) or not phrases
                or len(phrases) != len(set(phrases))
                or any(not p or p != " ".join(re.findall(r"[A-Za-z0-9]+", p)).lower()
                       for p in phrases)):
            raise ValueError("Malformed lexical effect category or phrase")
        ids.add(cid)
    review_rel = "reviews.json"
    review_doc = checked_json(safe_path(root, review_rel))
    review_source = checked_json(ROOT / "reviews/v1/claims.json")
    if (review_doc != review_source
            or review_doc.get("schema_version") != 1
            or review_doc.get("format") != "cheatarium-effect-reviews-v1"
            or not isinstance(review_doc.get("claims"), list)):
        raise ValueError("Unvalidated or changed effect review registry")
    for rel in ["catalog.json", "reports/snes-codec-coverage.json",
                taxonomy_rel, review_rel, composition_rel, queue_rel,
                "interpretations/snes-published-groups.json"]:
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
        repeat_rel = entry["repeat_index_artifact"]
        tags_rel = entry["tag_index_artifact"]
        if (tags_rel != f"tags/{platform}.json.gz"
                or source_rel != f"{platform}.json.gz"
                or game_rel != f"games/{platform}.json.gz"
                or repeat_rel != f"repeats/{platform}.json.gz"):
            raise ValueError(f"Unexpected artifact path for {platform}")
        identity_rel = entry.get("identity_artifact")
        artifacts = [source_rel, game_rel, repeat_rel, tags_rel]
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
        repeats = checked_json(safe_path(root, repeat_rel))
        tags = checked_json(safe_path(root, tags_rel))
        if (tags.get("schema_version") != 1
                or tags.get("platform") != platform
                or tags.get("taxonomy_id") != taxonomy["id"]
                or tags.get("interpretation") != "lexical-source-description-signal-only; no verified game effect or cartridge compatibility"
                or not isinstance(tags.get("categories"), list)):
            raise ValueError(f"Malformed lexical effect tag index: {platform}")
        if (repeats.get("schema_version") != 1
                or repeats.get("platform") != platform
                or not isinstance(repeats.get("groups"), list)
                or len(repeats["groups"]) != entry["repeat_groups"]):
            raise ValueError(f"Malformed raw-code repetition index: {platform}")
        if bundle["schema_version"] != 1 or bundle["platform"] != platform:
            raise ValueError(f"Bad source bundle header: {platform}")
        if games["schema_version"] != 1 or games["platform"] != platform:
            raise ValueError(f"Bad game index header: {platform}")
        records = bundle["records"]
        groups = games["candidates"]
        if len(records) != entry["source_files"] or len(groups) != entry["game_candidate_groups"]:
            raise ValueError(f"Incorrect record or group count: {platform}")

        record_ids = [r["id"] for r in records]
        by_record_id = {r["id"]: r for r in records}
        if len(record_ids) != len(set(record_ids)):
            raise ValueError(f"Duplicate source record IDs: {platform}")
        # Deterministic exact recomputation from original source descriptions.
        # Do not infer effect from filenames, code bytes or SNES decoding.
        expected_tags = {c["id"]: [] for c in categories}
        for record in records:
            for cheat in record["codes"]:
                if cheat["role"] not in ("code", "memory-entry"):
                    continue
                desc = cheat.get("description")
                if not isinstance(desc, str):
                    continue
                words = " " + " ".join(w.lower() for w in re.findall(r"[A-Za-z0-9]+", desc)) + " "
                for cat in categories:
                    found = next((phrase for phrase in cat["phrases"]
                                  if " " + phrase + " " in words), None)
                    if found is not None:
                        expected_tags[cat["id"]].append({
                            "source_record_id": record["id"],
                            "ordinal": cheat["ordinal"],
                            "candidate_game_key": record["candidate_game_key"],
                            "matched_phrase": found,
                        })
        expected_groups = [{"id": cid, "matches": found}
                           for cid, found in sorted(expected_tags.items()) if found]
        if tags["categories"] != expected_groups:
            raise ValueError(f"Effect tags disagree with source text or evidence rules: {platform}")
        count_tags = sum(len(group["matches"]) for group in expected_groups)
        if count_tags != entry["tag_matches"]:
            raise ValueError(f"Catalog effect-tag match count disagrees: {platform}")
        lexical_tag_matches += count_tags
        repeated_code_groups += len(repeats["groups"])
        repeat_keys = set()
        for group in repeats["groups"]:
            key = (group["candidate_game_key"], group.get("region_hint"),
                   group.get("revision_hint"), group.get("declared_format"),
                   group["source_code"])
            if key in repeat_keys:
                raise ValueError(f"Duplicate repetition grouping key: {platform}")
            repeat_keys.add(key)
            if (group.get("relation") != "identical-raw-code-text-within-advisory-filename-bucket"
                    or group.get("confirmed_equivalent_cheat") is not False
                    or group.get("verified_rom_compatibility") is not False):
                raise ValueError(f"Repetition result incorrectly claims verified cheat/ROM identity: {platform}")
            refs = group.get("occurrences", [])
            descriptions = {r["description"] for r in refs
                            if isinstance(r.get("description"), str)
                            and r["description"].strip()}
            if (group.get("description_variants") != len(descriptions)
                    or group.get("description_text_varies") is not (len(descriptions) > 1)):
                raise ValueError(f"Malformed description variation counts: {platform}")
            if not isinstance(refs, list) or len({r["source_record_id"] for r in refs}) < 2:
                raise ValueError(f"Repetition group lacks distinct source records: {platform}")
            seen_occurrences = set()
            for ref in refs:
                ordinal_key = (ref["source_record_id"], ref["ordinal"])
                if ordinal_key in seen_occurrences:
                    raise ValueError(f"Duplicated repetition reference: {platform}")
                seen_occurrences.add(ordinal_key)
                record = by_record_id.get(ref["source_record_id"])
                if record is None:
                    raise ValueError(f"Missing original repetition source record: {platform}")
                if (record["candidate_game_key"] != group["candidate_game_key"]
                        or record.get("region_hint") != group.get("region_hint")
                        or record.get("format_hint") != group.get("declared_format")):
                    raise ValueError(f"Repetition crossed candidate game, region or declared format: {platform}")
                # Re-read the explicit build marker, independently of the Rust indexer.
                tags = [
                    part.split(")", 1)[0].strip().lower()
                    for part in record["raw_filename"].split("(")[1:]
                    if ")" in part
                ]
                known_build_tags = [
                    tag for tag in tags
                    if tag.startswith(("rev ", "revision ", "version ", "beta",
                                       "proto", "demo", "v1.", "v2."))
                    or tag in ("unl", "virtual console")
                    or "hack" in tag or "translation" in tag
                ]
                if (known_build_tags[0] if known_build_tags else None) != group.get("revision_hint"):
                    raise ValueError(f"Repetition crosses a known revision/edition marker: {platform}")
                matching = [c for c in record["codes"] if c["ordinal"] == ref["ordinal"]]
                if (len(matching) != 1 or matching[0]["role"] != "code"
                        or matching[0].get("code") != group["source_code"]
                        or matching[0].get("description") != ref.get("description")):
                    raise ValueError(f"Repetition reference differs from original source code: {platform}")
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
        "repeated_raw_code_groups": repeated_code_groups,
        "lexical_effect_tag_matches": lexical_tag_matches,
        "reviewed_snes_revision_alternatives": compositions["reviewed_revision_alternatives"],
        "reviewed_snes_source_blobs_audited": compositions["reviewed_original_source_blobs_audited"],
        "reviewed_snes_source_ordinals_audited": compositions["reviewed_original_source_ordinals_audited"],
        "unresolved_snes_plus_groups": compositions["unresolved_plus_groups"],
        "unresolved_snes_candidate_groups": composition_queue["candidate_groups_with_unresolved_joins"],
        "historical_snes_publication_witnesses": witness_count,
        "historical_snes_source_blobs_audited": publication_witnesses["original_archived_files_checked"],
        "historical_snes_source_ordinals_audited": publication_witnesses["publication_witness_ordinals_checked"],
        "unresolved_snes_without_publication_witness": composition_queue["total_without_publication_witness"],
        "reviewed_effect_claims": len(review_doc["claims"]),
        "observed_effect_claims": sum(c.get("assessment") == "observed" for c in review_doc["claims"]),
        "effect_taxonomy": "cheatarium-effect-signals-en-v1",
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
