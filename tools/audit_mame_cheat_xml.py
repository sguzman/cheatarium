#!/usr/bin/env python3
"""Audit MAME 2010 cheat XML structure from its pinned ZIP without publishing cheats.

Source original is ephemeral. Original ZIP Git SHA-1 is verified against the
pinned link-only manifest, and only aggregate counts are output. Equality of
XML bytes or a serialized cheat node is not equality of gameplay effects.
"""
import argparse
from collections import Counter
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import xml.etree.ElementTree as ET
import zipfile

from audit_compiled_source_identity import read_verified, ROOT

FORMAT = "cheatarium-mame-xml-structural-audit-v1"
MAX_MEMBER_BYTES = 10_000_000
MAX_UNCOMPRESSED_TOTAL = 750_000_000
MAX_XML_FILES = 15000


def local_name(tag):
    if not isinstance(tag, str):
        return ""
    return tag.rpartition("}")[2] if "}" in tag else tag


def audit_zip(raw):
    count = Counter()
    members_by_content = Counter()
    cheats_by_structure = Counter()
    id_labels = Counter()
    roots = Counter()
    xml_schema_versions = Counter()
    total_uncompressed = 0
    per_file_max_cheats = 0
    parsed_files = 0
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        members = [m for m in archive.infolist() if not m.is_dir()]
        if len(members) > MAX_XML_FILES:
            raise ValueError("Unreasonable number of original ZIP members")
        total_uncompressed = sum(m.file_size for m in members)
        if total_uncompressed > MAX_UNCOMPRESSED_TOTAL:
            raise ValueError("Source ZIP uncompressed bounds exceeded")
        for item in members:
            name = item.filename.replace("\\", "/")
            path = PurePosixPath(name)
            if (name.startswith("/") or ".." in path.parts
                    or item.file_size > MAX_MEMBER_BYTES):
                raise ValueError("Untrusted archive path or excessive member size")
            count["member_file_occurrences"] += 1
            extension = path.suffix.lower()
            if extension != ".xml":
                count["non_xml_member_file_occurrences"] += 1
                continue
            count["xml_member_file_occurrences"] += 1
            id_labels[path.stem.casefold()] += 1
            contents = archive.read(item)
            if len(contents) != item.file_size:
                raise ValueError("ZIP declared bytes did not match source")
            members_by_content[hashlib.sha256(contents).digest()] += 1
            # Do not parse DTDs/entities from a third-party corpus.
            if b"<!DOCTYPE" in contents.upper() or b"<!ENTITY" in contents.upper():
                count["xml_rejected_doctype_or_entity"] += 1
                continue
            try:
                root = ET.fromstring(contents)
            except ET.ParseError:
                count["xml_parse_error_files"] += 1
                continue
            parsed_files += 1
            roots[local_name(root.tag)] += 1
            version = str(root.attrib.get("version", "<absent>"))
            if len(version) > 32:
                version = "<long-attribute>"
            xml_schema_versions[version] += 1
            cheats = 0
            for e in root.iter():
                tag = local_name(e.tag)
                if tag == "cheat":
                    cheats += 1
                    serialized = ET.tostring(e, encoding="utf-8")
                    cheats_by_structure[hashlib.sha256(serialized).digest()] += 1
                elif tag == "script":
                    count["script_element_occurrences"] += 1
                elif tag == "action":
                    count["action_element_occurrences"] += 1
                elif tag == "parameter":
                    count["parameter_element_occurrences"] += 1
            count["cheat_element_occurrences"] += cheats
            count["xml_files_with_cheat_element"] += cheats > 0
            per_file_max_cheats = max(per_file_max_cheats, cheats)

    return {
        "zip_member_occurrences": count["member_file_occurrences"],
        "xml_member_occurrences": count["xml_member_file_occurrences"],
        "non_xml_member_occurrences": count["non_xml_member_file_occurrences"],
        "uncompressed_source_member_bytes": total_uncompressed,
        "xml_members_rejected_for_doctype_or_entity": count["xml_rejected_doctype_or_entity"],
        "xml_parse_error_members": count["xml_parse_error_files"],
        "xml_members_parsed": parsed_files,
        "source_game_filename_stem_labels": len(id_labels),
        "same_filename_stem_excess_xml_members": sum(id_labels.values()) - len(id_labels),
        "xml_files_with_cheat_elements": count["xml_files_with_cheat_element"],
        "cheat_element_occurrences": count["cheat_element_occurrences"],
        "max_cheat_elements_in_one_xml_member": per_file_max_cheats,
        "script_element_occurrences": count["script_element_occurrences"],
        "action_element_occurrences": count["action_element_occurrences"],
        "parameter_element_occurrences": count["parameter_element_occurrences"],
        "distinct_exact_member_file_bytes": len(members_by_content),
        "repeated_exact_xml_member_byte_occurrences":
            count["xml_member_file_occurrences"] - len(members_by_content),
        "distinct_exact_serialized_cheat_xml_subtrees": len(cheats_by_structure),
        "repeated_exact_serialized_cheat_subtree_occurrences":
            count["cheat_element_occurrences"] - len(cheats_by_structure),
        "source_root_element_tag_counts": dict(sorted(roots.items())),
        "source_root_version_attribute_counts": dict(sorted(xml_schema_versions.items())),
        "individual_game_labels_cheat_text_xml_or_fingerprints_exported": False,
    }


def build():
    source = json.loads(
        (ROOT / "sources/external-bulk/libretro-mame2010-cheat-archive.json")
        .read_text("utf-8")
    )
    if (source.get("id") != "libretro-mame2010-cheat-archive"
            or source.get("platform") != "arcade-mame"
            or source.get("rights_status") != "link_only_rights_review"
            or len(source.get("files", [])) != 1):
        raise ValueError("Unexpected MAME source scope or rights")
    payload = read_verified(source, source["files"][0])
    inspection = audit_zip(payload)
    if inspection["zip_member_occurrences"] != 8837:
        raise ValueError("Previous source XML member inventory no longer reconciles")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "pinned-git-blob-verified-original-zip-ephemeral-read",
        "source_id": "libretro-mame2010-cheat-archive",
        "original_zip_file_occurrences": 1,
        "original_zip_sha1_verified": True,
        "no_original_code_xml_or_identifiers_exported": True,
        "no_source_cheat_body_copied_into_repo": True,
        "redistribution_rights_cleared": False,
        "gameplay_effects_verified": False,
        "game_and_revision_identity_verified": False,
        "caveat": "Distinct exact XML subtrees do not mean distinct or working code effects, titles or game revisions.",
        "structure": inspection,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    result = build()
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", "utf-8")
    print("BEGIN_MAME_XML_AUDIT")
    print(json.dumps(result, indent=2, sort_keys=True))
    print("END_MAME_XML_AUDIT")


if __name__ == "__main__":
    main()
