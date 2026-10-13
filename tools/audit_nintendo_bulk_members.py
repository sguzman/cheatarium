#!/usr/bin/env python3
"""Count pinned Nintendo GameCube WiiRD and Wii U native cheat records ephemerally.

Original code texts, ZIP member paths, names, and fingerprint values never enter
the output. All third-party original file bytes are verified against their
pinned Git blob SHA-1. This audit is not an author license or gameplay test.
"""
import argparse
from collections import Counter
import hashlib
import io
import json
from pathlib import PurePosixPath, Path
import re
import xml.etree.ElementTree as ET
import zipfile

from audit_compiled_source_identity import read_verified, ROOT

FORMAT = "cheatarium-nintendo-bulk-member-coverage-v1"
MAX_MEMBER_BYTES = 5_000_000
MAX_UNPACKED_ZIP_BYTES = 300_000_000
MAX_ZIP_MEMBERS = 40000


def analyze_gamecube_zip_contents(source_files):
    """List in-memory central directories; hash members for byte-only alias counts."""
    regional = []
    unique_member_bodies = set()
    byte_occurrences = 0
    all_members = 0
    all_suffixes = Counter()
    source_names = Counter()
    for outer_path, raw_bytes in source_files:
        member_hashes = set()
        suffixes = Counter()
        repeated_member_names = Counter()
        with zipfile.ZipFile(io.BytesIO(raw_bytes)) as z:
            infos = [i for i in z.infolist() if not i.is_dir()]
            if (len(infos) > MAX_ZIP_MEMBERS or
                    sum(i.file_size for i in infos) > MAX_UNPACKED_ZIP_BYTES):
                raise ValueError("Unsafe source ZIP member inventory size")
            for info in infos:
                name = info.filename.replace("\\", "/")
                p = PurePosixPath(name)
                if (name.startswith("/") or ".." in p.parts
                        or info.file_size > MAX_MEMBER_BYTES):
                    raise ValueError("Unsafe ZIP member path or member size")
                extension = p.suffix.lower() or "<none>"
                suffixes[extension] += 1
                repeated_member_names[name.casefold()] += 1
                body = z.read(info)
                if len(body) != info.file_size:
                    raise ValueError("Corrupt ZIP member length")
                sha = hashlib.sha256(body).digest()
                member_hashes.add(sha)
                unique_member_bodies.add(sha)
                all_members += 1
                all_suffixes[extension] += 1
                byte_occurrences += len(body)
                # Compare names internally but never publish member labels.
                source_names[name.casefold()] += 1
        regional.append({
            "original_zip_name": outer_path,
            "member_file_occurrences": sum(suffixes.values()),
            "distinct_member_body_bytes": len(member_hashes),
            "repeated_member_byte_occurrences_within_zip": sum(suffixes.values()) - len(member_hashes),
            "member_extension_occurrences": dict(sorted(suffixes.items())),
            "duplicate_casefolded_member_names_within_zip": sum(v - 1 for v in repeated_member_names.values()),
        })
    return {
        "source_regional_zip_containers": len(source_files),
        "regional_zip_counts": sorted(regional, key=lambda x:x["original_zip_name"]),
        "total_member_file_occurrences": all_members,
        "distinct_exact_member_byte_bodies_across_all_regions": len(unique_member_bodies),
        "repeated_exact_member_byte_occurrences": all_members - len(unique_member_bodies),
        "same_member_path_occurrences_across_regions": sum(v - 1 for v in source_names.values()),
        "original_uncompressed_member_bytes": byte_occurrences,
        "member_extension_occurrences": dict(sorted(all_suffixes.items())),
        "source_file_names_are_region_container_names_not_game_ids": True,
        "game_title_or_rom_revision_verified": False,
        "member_names_hashes_and_code_text_not_exported": True,
    }


def local_name(tag):
    return tag.rsplit("}", 1)[-1] if isinstance(tag, str) else ""


def analyze_wiiu_xml(sources):
    totals = Counter()
    hashes = set()
    credit_labels = set()
    games = 0
    for path, raw in sources:
        # stdlib ElementTree does not resolve external entities, but reject
        # any declared DTD or entity to keep the input surface minimal.
        try:
            text = raw.decode("utf-16") if raw.startswith((b"\xff\xfe", b"\xfe\xff")) else raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise ValueError("Unsupported source XML encoding") from exc
        if "<!DOCTYPE" in text.upper() or "<!ENTITY" in text.upper():
            raise ValueError("External XML entity declarations are not allowed")
        # A few community Wii U XML dumps announce UTF-16 but are actually
        # UTF-8 bytes without a BOM. Decode the bytes first, drop the unreliable
        # encoding declaration and parse the recovered Unicode text. Never
        # silently replace invalid source bytes.
        normalized = re.sub(r'^\\s*<\\?xml[^>]*\\?>', '', text, count=1, flags=re.I)
        try:
            root = ET.fromstring(normalized)
        except ET.ParseError as exc:
            raise ValueError("Broken original Wii U XML") from exc
        games += 1
        entries = [x for x in root.iter() if local_name(x.tag) == "entry"]
        totals["entry_occurrences"] += len(entries)
        for entry in entries:
            code = next((x.text or "" for x in entry if local_name(x.tag) == "code"), "")
            if code.strip():
                totals["entry_occurrences_with_code_text"] += 1
                key = hashlib.sha256(code.strip().encode("utf-8")).digest()
                hashes.add(key)
                totals["nonempty_code_line_occurrences"] += len([y for y in code.splitlines() if y.strip()])
            author = next((x.text or "" for x in entry if local_name(x.tag) in ("authors", "author")), "")
            if author.strip():
                totals["entries_with_author_label"] += 1
                credit_labels.add(author.strip())
            if any(local_name(x.tag) == "codeInputType" for x in entry):
                totals["entries_with_format_hint"] += 1
    totals["distinct_exact_nonempty_code_text_bodies"] = len(hashes)
    totals["repeated_nonempty_code_body_occurrences"] = (
        totals["entry_occurrences_with_code_text"] - len(hashes)
    )
    totals["distinct_nonempty_author_label_strings"] = len(credit_labels)
    return {
        "original_xml_source_files": games,
        **dict(sorted(totals.items())),
        "original_codes_game_names_credit_strings_or_hashes_exported": False,
        "author_labels_are_not_copyright_permissions": True,
    }


def analyze_wiiu_json(sources):
    total = Counter()
    all_bodies = set()
    original_blob_hashes = set()
    per_original = []
    for path, sha, raw in sources:
        original_blob_hashes.add(sha)
        try:
            body = json.loads(raw)
        except (ValueError, UnicodeError) as exc:
            raise ValueError("Unparseable Wii U JSON") from exc
        if not isinstance(body, list):
            raise ValueError("Expected Wii U cheat record array")
        local = Counter()
        for rec in body:
            if not isinstance(rec, dict):
                raise ValueError("Unrecognized Wii U cheat record")
            local["record_occurrences"] += 1
            total["record_occurrences"] += 1
            if str(rec.get("author", "")).strip():
                local["record_occurrences_with_author_label"] += 1
                total["record_occurrences_with_author_label"] += 1
            if rec.get("cafe_code", {}).get("code"):
                local["cafe_code_record_occurrences"] += 1
                total["cafe_code_record_occurrences"] += 1
            if rec.get("ugecko", {}).get("code"):
                local["ugecko_code_record_occurrences"] += 1
                total["ugecko_code_record_occurrences"] += 1
            canonical = json.dumps((rec.get("cafe_code"), rec.get("ugecko")), sort_keys=True)
            all_bodies.add(hashlib.sha256(canonical.encode("utf-8")).digest())
        per_original.append({
            "original_json_source_path": path,
            "record_occurrences": local["record_occurrences"],
            "record_occurrences_with_author_label": local["record_occurrences_with_author_label"],
            "cafe_code_record_occurrences": local["cafe_code_record_occurrences"],
            "ugecko_code_record_occurrences": local["ugecko_code_record_occurrences"],
        })
    return {
        "original_json_source_file_occurrences": len(sources),
        "distinct_original_git_file_blobs": len(original_blob_hashes),
        "per_file_source_record_counts": sorted(per_original, key=lambda x:x["original_json_source_path"]),
        **dict(sorted(total.items())),
        "distinct_exact_dual_format_code_payload_pairs": len(all_bodies),
        "repeated_dual_format_payload_pair_occurrences": total["record_occurrences"] - len(all_bodies),
        "individual_codes_names_authors_or_code_hashes_exported": False,
        "root_website_license_does_not_clear_third_party_codes": True,
    }


def build():
    def load(sid):
        o = json.loads((ROOT / "sources/external-bulk" / (sid + ".json")).read_text("utf-8"))
        if (o.get("id") != sid or o.get("rights_status") != "link_only_rights_review"
                or o.get("file_counts", {}).get("cheat") != len(o.get("files", []))):
            raise ValueError("Unexpected Nintendo source or rights state")
        return o

    gc = load("leumas-gamecube-wiird-zips")
    ku = load("katsumaki-wiiu-xml")
    uc = load("ucodes-wiiu-json")
    if len(gc["files"]) != 3 or len(ku["files"]) != 9 or len(uc["files"]) != 4:
        raise ValueError("Pinned regional GameCube and Wii U input counts changed")
    gc_result = analyze_gamecube_zip_contents([
        (row[0], read_verified(gc, row)) for row in gc["files"]
    ])
    ku_result = analyze_wiiu_xml([
        (row[0], read_verified(ku, row)) for row in ku["files"]
    ])
    uc_result = analyze_wiiu_json([
        (row[0], row[1], read_verified(uc, row)) for row in uc["files"]
    ])
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "ephemeral-original-git-sha1-verified-native-container-audit",
        "new_file_reference_occurrences": 16,
        "new_distinct_git_blobs": 15,
        "game_and_build_compatibility_verified": False,
        "original_cheat_code_contents_committed": False,
        "individual_cheat_code_effects_verified": False,
        "third_party_reuse_permissions_established": False,
        "gamecube_wiird_zip": gc_result,
        "wiiu_xml": ku_result,
        "wiiu_json": uc_result,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = build()
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True) + "\n", "utf-8")
    print("BEGIN_NINTENDO_BULK_MEMBER_AUDIT")
    print(json.dumps(result, indent=2, ensure_ascii=False, sort_keys=True))
    print("END_NINTENDO_BULK_MEMBER_AUDIT")


if __name__ == "__main__":
    main()
