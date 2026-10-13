#!/usr/bin/env python3
"""Compare pinned MAME 2003-Plus cheat.dat driver labels to MAME 2010 XML names.

Reads exact verified original bytes ephemerally. Only aggregate counts are
exported: neither cheat text, driver-name lists nor fingerprint values.
Driver-label equality is NOT verified compatible game or cheat behavior.
"""
import argparse
from collections import Counter
import io
import json
from pathlib import Path, PurePosixPath
import re
import zipfile

from audit_compiled_source_identity import ROOT, read_verified

FORMAT = "cheatarium-mame-dat-xml-driver-label-comparison-v1"
DATA_LINE = re.compile(
    r"^:([^:\s]{1,80}):([a-fA-F0-9]{8}):([a-fA-F0-9]{1,8}):"
    r"([a-fA-F0-9]{1,8}):([a-fA-F0-9]{1,8}):"
)


def parse_dat(raw):
    driver_occurrences = Counter()
    exact_lines = set()
    raw_line_count = 0
    nonempty = 0
    comments = 0
    native_lines = 0
    malformed_candidate_lines = 0
    internal_commands = 0
    dat = raw.decode("latin-1").splitlines()
    for raw_line in dat:
        raw_line_count += 1
        text = raw_line.strip()
        if not text:
            continue
        nonempty += 1
        if text.startswith(";") or text.startswith("#"):
            comments += 1
            continue
        if text.startswith(":_command:"):
            internal_commands += 1
            continue
        match = DATA_LINE.match(text)
        if match:
            driver = match.group(1).casefold()
            driver_occurrences[driver] += 1
            native_lines += 1
            exact_lines.add(text)
        elif text.startswith(":"):
            malformed_candidate_lines += 1
    return {
        "driver_labels": set(driver_occurrences),
        "source_line_occurrences": raw_line_count,
        "nonempty_lines": nonempty,
        "comment_lines": comments,
        "native_cheat_dat_record_occurrences": native_lines,
        "distinct_exact_native_record_text_lines": len(exact_lines),
        "distinct_native_driver_labels": len(driver_occurrences),
        "duplicate_native_record_text_occurrences": native_lines - len(exact_lines),
        "internal_command_lines": internal_commands,
        "unparsed_colon_line_occurrences": malformed_candidate_lines,
    }


def compare_dat_xml(raw_dat, raw_zip):
    parsed = parse_dat(raw_dat)
    with zipfile.ZipFile(io.BytesIO(raw_zip)) as f:
        names = [x.filename.replace("\\", "/") for x in f.infolist() if not x.is_dir()]
    if len(names) > 15_000:
        raise ValueError("Pinned original MAME ZIP exceeds expected member scope")
    xml_labels = set()
    for name in names:
        path = PurePosixPath(name)
        if name.startswith("/") or ".." in path.parts or path.suffix.lower() != ".xml":
            raise ValueError("Unexpected MAME 2010 ZIP member path or format")
        xml_labels.add(path.stem.casefold())
    dat_labels = parsed.pop("driver_labels")
    shared = dat_labels & xml_labels
    return {
        "dat": parsed,
        "xml_member_filename_occurrences": len(names),
        "xml_distinct_filename_driver_labels": len(xml_labels),
        "distinct_driver_labels_in_both_dat_and_xml": len(shared),
        "distinct_driver_labels_only_in_dat": len(dat_labels - xml_labels),
        "distinct_driver_labels_only_in_xml": len(xml_labels - dat_labels),
        "driver_label_union": len(dat_labels | xml_labels),
        "exact_game_release_or_code_equivalence_verified": False,
        "individual_driver_labels_code_text_or_hashes_exported": False,
        "note": "Source-native MAME driver filename/record labels only; equal labels do not authenticate arcade ROM sets or cheat equivalence.",
    }


def build():
    root = ROOT / "sources/external-bulk"
    items = {}
    for sid in ("libretro-mame2003plus-cheat-archive", "libretro-mame2010-cheat-archive"):
        source = json.loads((root / (sid + ".json")).read_text("utf-8"))
        if (source.get("id") != sid or source.get("platform") != "arcade-mame"
                or source.get("rights_status") != "link_only_rights_review"
                or len(source.get("files", [])) != 1):
            raise ValueError("Malformed pinned MAME provenance or rights flags")
        items[sid] = read_verified(source, source["files"][0])
    report = compare_dat_xml(
        items["libretro-mame2003plus-cheat-archive"],
        items["libretro-mame2010-cheat-archive"],
    )
    if report["xml_member_filename_occurrences"] != 8837:
        raise ValueError("MAME XML source member count differs from verified baseline")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "method": "ephemeral-pinned-git-blob-sha1-verified-native-driver-label-compare",
        "rights_to_redistribute_established": False,
        "source_cheat_bodies_exported_or_committed": False,
        "original_git_blob_checksums_verified": True,
        "source_catalogs": ["libretro-mame2003plus-cheat-archive", "libretro-mame2010-cheat-archive"],
        "comparison": report,
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    result = build()
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", "utf-8")
    print("BEGIN_MAME_DAT_VS_XML")
    print(json.dumps(result, indent=2, sort_keys=True))
    print("END_MAME_DAT_VS_XML")


if __name__ == "__main__":
    main()
