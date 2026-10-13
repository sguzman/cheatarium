#!/usr/bin/env python3
"""List GameCube regional WiiRD ZIP member names with original container provenance.

Outputs ONLY filenames, sizes, region labels and pinned upstream ZIP URLs.
Never emits or stores cheat code bytes, game image bytes or per-code hashes.
These source-provided filename/title hints do not verify ROM identity.
"""
import argparse
import io
import json
from pathlib import Path, PurePosixPath
import re
import urllib.parse
import zipfile

from audit_compiled_source_identity import ROOT, read_verified

FORMAT = "cheatarium-nintendo-gamecube-wiird-member-references-v1"
REGIONS = {
    "NTSC-J codes.zip": "NTSC-J",
    "NTSC-U Codes.zip": "NTSC-U",
    "PAL codes.zip": "PAL",
}
MAX_SIZE = 5_000_000


def index_archive(source, outer, body):
    name, git_blob_sha, _size = outer
    if name not in REGIONS:
        raise ValueError("Unexpected regional ZIP name")
    url = (source["repository"] + "/blob/" + source["snapshot_commit"] + "/" +
           urllib.parse.quote(name, safe="/"))
    with zipfile.ZipFile(io.BytesIO(body)) as archive:
        entries = [entry for entry in archive.infolist() if not entry.is_dir()]
        if len(entries) > 5000 or sum(x.file_size for x in entries) > 300_000_000:
            raise ValueError("Unexpected ZIP member inventory")
        results = []
        names = set()
        for entry in entries:
            name_in_zip = entry.filename.replace("\\", "/")
            path = PurePosixPath(name_in_zip)
            if (name_in_zip.startswith("/") or "\x00" in name_in_zip or
                    ".." in path.parts or path.suffix.lower() != ".txt"
                    or entry.file_size > MAX_SIZE):
                raise ValueError("Unsafe or unsupported GameCube ZIP member name")
            if name_in_zip in names:
                raise ValueError("Duplicate ZIP member filename")
            names.add(name_in_zip)
            # This deliberately does not read decompressed cheat text.
            results.append({
                "region": REGIONS[name],
                "original_container_path": name,
                "original_container_git_blob_sha1": git_blob_sha,
                "original_container_url": url,
                "native_zip_member_path": name_in_zip,
                "source_filename_title_hint": path.stem,
                "source_member_uncompressed_bytes": entry.file_size,
            })
    return results


def collect(sources):
    refs = []
    for source, row, raw in sources:
        refs.extend(index_archive(source, row, raw))
    refs.sort(key=lambda x: (x["region"], x["native_zip_member_path"].casefold(),
                             x["original_container_path"]))
    keys = {(x["region"], x["native_zip_member_path"]) for x in refs}
    if len(keys) != len(refs):
        raise ValueError("Source region/member references repeat")
    return {
        "schema_version": 1,
        "format": FORMAT,
        "platform": "gamecube",
        "provenance_scope": "GameCube WiiRD region-native original ZIP member name claims",
        "read_only_filename_metadata": True,
        "rights_status": "link_only_rights_review",
        "cheat_bodies_included": False,
        "original_file_bytes_verified": True,
        "game_identifiers_or_revisions_confirmed": False,
        "gameplay_tested": False,
        "region_count": 3,
        "reference_count": len(refs),
        "file_occurrences_by_region": {
            name: sum(x["region"] == name for x in refs)
            for name in ("NTSC-J", "NTSC-U", "PAL")
        },
        "records": refs,
    }


def build():
    src = json.loads(
        (ROOT / "sources/external-bulk/leumas-gamecube-wiird-zips.json")
        .read_text("utf-8")
    )
    if (src.get("id") != "leumas-gamecube-wiird-zips"
            or src.get("rights_status") != "link_only_rights_review"
            or len(src.get("files", [])) != 3):
        raise ValueError("Unrecognized upstream Nintendo rights/source manifest")
    sources = [(src, row, read_verified(src, row)) for row in src["files"]]
    report = collect(sources)
    if report["file_occurrences_by_region"] != {
        "NTSC-J": 8, "NTSC-U": 138, "PAL": 229
    }:
        raise ValueError("Regional original source member scope drift")
    return report


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    result = build()
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", "utf-8")
    print("BEGIN_GAMECUBE_WIIRD_MEMBER_REFERENCES")
    print(json.dumps(result, ensure_ascii=False))
    print("END_GAMECUBE_WIIRD_MEMBER_REFERENCES")


if __name__ == "__main__":
    main()
