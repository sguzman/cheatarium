#!/usr/bin/env python3
"""Build/verify a PRIVATE normalized Wii Gecko sidecar for local consumers.

Reads the full pinned 29-file cache, verifies its original Git blobs, and
projects the 45 source-native groups into a private JSON file. Never executes,
applies or enables the codes. Never writes third-party code text into Git, CI
artifacts or packaged Cheatarium distribution.
"""
import argparse
import json
import os
from pathlib import Path
import tempfile

from hydrate_wii_bulk import default_destination
from query_wii_gecko_local import verified_index

ROOT = Path(__file__).resolve().parents[1]
FORMAT = "cheatarium.private_wii_gecko_sidecar.v1"
DEFAULT_OUTPUT = Path(
    os.environ.get("XDG_DATA_HOME", str(Path.home() / ".local/share"))
) / "cheatarium" / "wii-local-sidecar" / "codes.json"


def build_document(entries):
    records = []
    for source in entries:
        for group in source["groups"]:
            if not group["code_lines"] or group["instruction_pair_count"] != len(group["code_lines"]):
                raise ValueError("Wii native Gecko group lacks valid original instructions")
            records.append({
                "platform": "wii",
                "claimed_disc_id": source["claimed_disc_id"],
                "source_revision_token": source["source_revision_token"],
                "source_variant_token": source["source_variant_token"],
                "game_family_title_hint": source["game_family_title_hint"],
                "source_compatibility_warning": source["source_compatibility_warning"],
                "original_source_id": source["source_id"],
                "original_local_path": source["local_path"],
                "original_git_blob_sha1": source["original_git_blob_sha1"],
                "original_upstream_url": source["upstream_original_url"],
                "original_group_index": group["group_index"],
                "source_group_title": group["title"],
                "source_notes": group["notes"],
                "source_marked_enabled": group["source_enabled"],
                "code_format": "gecko-native-eight-hex-word-pairs",
                "native_code_lines": group["code_lines"],
                "native_code_pair_count": group["instruction_pair_count"],
                "enabled_for_execution": False,
                "game_build_authenticated": False,
                "gameplay_verified": False,
                "original_author_redistribution_rights_cleared": False,
            })
    if len(entries) != 29 or len(records) != 45:
        raise ValueError("Original Wii native code inventory unexpectedly changed")
    if sum(x["native_code_pair_count"] for x in records) != 5611:
        raise ValueError("Unexpected original Wii Gecko instruction-pair inventory")
    records.sort(key=lambda r: (
        r["claimed_disc_id"],
        r["source_revision_token"] or "",
        r["source_variant_token"],
        r["original_source_id"],
        r["original_group_index"],
    ))
    keys = {(r["original_source_id"], r["original_local_path"], r["original_group_index"])
            for r in records}
    if len(keys) != 45:
        raise ValueError("Duplicate native Wii group identity")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "platform": "wii",
        "source_verified_original_files": 29,
        "source_native_group_count": 45,
        "native_instruction_pair_count": 5611,
        "game_builds_verified": 0,
        "gameplay_tested_effects": 0,
        "all_groups_disabled_by_default": True,
        "private_source_codes_present": True,
        "public_archive_import": False,
        "third_party_code_redistribution_rights_cleared": False,
        "origin": "on_demand_local_verified_cheatarium_wii_source_cache",
        "records": records,
    }


def encoded_document(cache):
    return (
        json.dumps(build_document(verified_index(cache)), ensure_ascii=False,
                   indent=2, sort_keys=True) + "\n"
    ).encode("utf-8")


def safe_output_path(path, root=ROOT):
    path = Path(path).expanduser().absolute()
    resolved = path.resolve()
    if resolved == root.resolve() or root.resolve() in resolved.parents:
        raise ValueError("Refusing to write third-party Wii source codes into Cheatarium's public Git tree")
    if path.suffix.lower() != ".json":
        raise ValueError("Sidecar output path must end with .json")
    if path.parent == path:
        raise ValueError("Invalid Wii sidecar output")
    return path


def write_sidecar(path, payload, *, replace=False, root=ROOT):
    path = safe_output_path(path, root=root)
    if path.exists() and not replace:
        raise FileExistsError("Existing Wii sidecar will not be silently overwritten; use --replace")
    path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    if path.is_symlink():
        raise ValueError("Refusing to replace a symlink with third-party Wii code content")
    with tempfile.NamedTemporaryFile(mode="wb", dir=path.parent, prefix=".wii-gecko-sidecar-",
                                     suffix=".tmp", delete=False) as tmp:
        tmp_path = Path(tmp.name)
        try:
            os.chmod(tmp_path, 0o600)
            tmp.write(payload)
            tmp.flush()
            os.fsync(tmp.fileno())
        except BaseException:
            tmp_path.unlink(missing_ok=True)
            raise
    try:
        if path.exists() and not replace:
            raise FileExistsError("Wii sidecar created concurrently")
        os.replace(tmp_path, path)
    finally:
        tmp_path.unlink(missing_ok=True)
    return path


def verify_sidecar(path, payload, *, root=ROOT):
    path = safe_output_path(path, root=root)
    if path.is_symlink() or not path.is_file():
        raise ValueError("Private Wii sidecar missing or is a symlink")
    if path.read_bytes() != payload:
        raise ValueError("Private Wii sidecar differs from exact pinned original source projection")
    return path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--write", action="store_true", help="Write private local Wii Gecko sidecar")
    action.add_argument("--verify", action="store_true", help="Verify existing sidecar offline against pinned originals")
    parser.add_argument("--cache", type=Path, default=default_destination())
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--replace", action="store_true",
                        help="Explicitly replace an already existing local sidecar")
    args = parser.parse_args()
    if args.verify and args.replace:
        parser.error("--replace applies only to --write")
    try:
        payload = encoded_document(args.cache)
        if args.write:
            destination = write_sidecar(args.output, payload, replace=args.replace)
        else:
            destination = verify_sidecar(args.output, payload)
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    # Do not print original codes, titles, author notes or per-game hashes:
    print(json.dumps({
        "output": str(destination),
        "source_files_verified": 29,
        "source_native_groups": 45,
        "instruction_pairs": 5611,
        "original_codes_private": True,
        "all_disabled_for_execution": True,
        "gameplay_verified": False,
        "rights_to_redistribute_original_codes": False,
    }, indent=2))


if __name__ == "__main__":
    main()
