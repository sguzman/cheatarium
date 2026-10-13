#!/usr/bin/env python3
"""Ephemeral pinned bulk-source comparison; only aggregated, non-code counts leave memory.

Downloads specifically pinned original upstream bytes, validates Git blob hashes,
and compares source-declared game identifiers/code text. It does not export
cheat bodies, game-title lists, individual code fingerprints, or game images.
"""
import argparse
from collections import Counter
import hashlib
import io
import json
import pathlib
import re
import urllib.parse
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
FORMAT = "cheatarium-compiled-source-overlap-v1"
MAX_BYTES = 100_000_000
MAX_ZIP_MEMBER_BYTES = 1_000_000


def read_verified(source, row):
    path, expected_sha, expected_bytes = row
    repo = source["repository"].removeprefix("https://github.com/")
    rev = source["snapshot_commit"]
    if (not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo)
            or not re.fullmatch(r"[0-9a-f]{40}", rev)
            or not re.fullmatch(r"[0-9a-f]{40}", expected_sha)
            or not isinstance(expected_bytes, int)
            or expected_bytes < 0 or expected_bytes > MAX_BYTES
            or not isinstance(path, str) or path.startswith("/")
            or any(x in ("", "..", ".") for x in path.split("/"))):
        raise ValueError("Invalid pinned Git source identity")
    url = (f"https://raw.githubusercontent.com/{repo}/{rev}/" +
           urllib.parse.quote(path, safe="/"))
    request = urllib.request.Request(url, headers={
        "User-Agent": "Cheatarium-ephemeral-identity-audit",
        "Accept": "application/octet-stream",
    })
    with urllib.request.urlopen(request, timeout=90) as response:
        data = response.read(expected_bytes + 1)
    if len(data) != expected_bytes:
        raise ValueError("Git source byte length mismatch")
    actual = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    if actual != expected_sha:
        raise ValueError("Git source blob identity mismatch")
    return data


def parse_cwcheat(raw):
    serial = None
    section_count = 0
    cheat_label_count = 0
    command_count = 0
    groups = []
    commands = []
    open_cheat = False

    def finish():
        if open_cheat and serial and commands:
            groups.append((serial, tuple(commands)))

    serials = set()
    for line in raw.decode("utf-8", "replace").splitlines():
        line = line.strip()
        if line.startswith("_S "):
            finish()
            commands = []
            open_cheat = False
            serial = line[3:].strip().upper()
            if serial:
                serials.add(serial)
            section_count += 1
        elif re.match(r"^_C[01]\s", line):
            finish()
            commands = []
            open_cheat = True
            cheat_label_count += 1
        elif line.startswith("_L ") and open_cheat:
            commands.append(line[3:].strip())
            command_count += 1
    finish()
    return {
        "serials": serials,
        "groups": groups,
        "source_game_section_occurrences": section_count,
        "cheat_group_label_occurrences": cheat_label_count,
        "native_code_command_occurrences": command_count,
    }


def compare_cwcheat(left, right):
    a, b = parse_cwcheat(left), parse_cwcheat(right)
    groups_a, groups_b = set(a["groups"]), set(b["groups"])
    shared_serials = a["serials"] & b["serials"]
    overlap = groups_a & groups_b
    result = {
        "source_files": 2,
        "a": {
            "declared_game_sections": a["source_game_section_occurrences"],
            "distinct_game_serial_labels": len(a["serials"]),
            "cheat_group_header_occurrences": a["cheat_group_label_occurrences"],
            "native_command_line_occurrences": a["native_code_command_occurrences"],
            "code_bearing_group_occurrences": len(a["groups"]),
            "distinct_serial_and_exact_code_block_pairs": len(groups_a),
        },
        "b": {
            "declared_game_sections": b["source_game_section_occurrences"],
            "distinct_game_serial_labels": len(b["serials"]),
            "cheat_group_header_occurrences": b["cheat_group_label_occurrences"],
            "native_command_line_occurrences": b["native_code_command_occurrences"],
            "code_bearing_group_occurrences": len(b["groups"]),
            "distinct_serial_and_exact_code_block_pairs": len(groups_b),
        },
        "same_serial_labels_in_both_sources": len(shared_serials),
        "serial_labels_only_in_a": len(a["serials"] - b["serials"]),
        "serial_labels_only_in_b": len(b["serials"] - a["serials"]),
        "exact_serial_and_code_block_pairs_in_both_sources": len(overlap),
        "distinct_exact_serial_and_code_block_pairs_in_union": len(groups_a | groups_b),
        "code_block_pairs_only_in_a": len(groups_a - groups_b),
        "code_block_pairs_only_in_b": len(groups_b - groups_a),
        "comparison_scope": "same source-declared serial plus exact normalized _L lines; cheat names ignored",
        "individual_serials_codes_or_hashes_exported": False,
    }
    return result


def summarize_nintendo_exports(blobs):
    results = {}
    for platform in ("switch", "3ds", "nes", "wiiu"):
        payload = json.loads(blobs[platform])
        if not isinstance(payload, list):
            raise ValueError("Unexpected Nintendo export JSON structure")
        labels = set()
        nonempty = 0
        codes = 0
        code_strings = 0
        code_arrays = 0
        seen_titles = set()
        with_build_id = 0
        code_only = set()
        repeated_exact_code_by_game = 0
        for game in payload:
            if not isinstance(game, dict) or not isinstance(game.get("cheat"), list):
                raise ValueError("Missing source-declared game/cheat fields")
            gid = str(game.get("id", ""))
            labels.add(gid)
            seen_titles.add(str(game.get("name", "")))
            if game.get("build_id"):
                with_build_id += 1
            if game["cheat"]:
                nonempty += 1
            for item in game["cheat"]:
                if not isinstance(item, dict):
                    raise ValueError("Malformed cheat export item")
                code = item.get("code")
                if isinstance(code, str):
                    code_strings += 1
                    code_key = (code,)
                elif isinstance(code, list) and all(isinstance(x, str) for x in code):
                    code_arrays += 1
                    code_key = tuple(code)
                else:
                    raise ValueError("Unrecognized Nintendo code field type")
                codes += 1
                fp = (gid, code_key)
                if fp in code_only:
                    repeated_exact_code_by_game += 1
                code_only.add(fp)
        results[platform] = {
            "source_game_or_variant_records": len(payload),
            "source_game_records_with_cheats": nonempty,
            "distinct_source_game_id_labels": len(labels),
            "distinct_source_title_texts": len(seen_titles),
            "game_records_with_build_id_field": with_build_id,
            "cheat_record_occurrences": codes,
            "cheat_records_with_code_string": code_strings,
            "cheat_records_with_code_line_array": code_arrays,
            "repeat_exact_code_text_within_same_id_label": repeated_exact_code_by_game,
            "individual_identifiers_titles_or_codes_exported": False,
        }
    return results


def sega_zip_members(files):
    # The data passed here consists of verified original ZIP sources only.
    seen_zip = set()
    member_hash_occurrences = Counter()
    suffix_counts = Counter()
    file_occurrences = 0
    credits_txt = 0
    for _path, outer_sha, body in files:
        if outer_sha in seen_zip:
            continue
        seen_zip.add(outer_sha)
        with zipfile.ZipFile(io.BytesIO(body)) as z:
            members = [x for x in z.infolist() if not x.is_dir()]
            if len(members) > 30000 or sum(x.file_size for x in members) > 60_000_000:
                raise ValueError("ZIP member limits exceeded")
            for info in members:
                name = info.filename.replace("\\", "/")
                if (name.startswith("/") or ".." in pathlib.PurePosixPath(name).parts
                        or info.file_size > MAX_ZIP_MEMBER_BYTES):
                    raise ValueError("Unsafe or oversized ZIP member")
                ext = pathlib.PurePosixPath(name).suffix.lower() or "<none>"
                content = z.read(info)
                digest = hashlib.sha256(content).digest()
                member_hash_occurrences[digest] += 1
                suffix_counts[ext] += 1
                file_occurrences += 1
                if ext == ".txt" and "credit" in content.decode("utf-8", "replace").lower():
                    credits_txt += 1
    return {
        "distinct_original_zip_git_blobs": len(seen_zip),
        "member_file_occurrences_excluding_identical_zip_aliases": file_occurrences,
        "distinct_exact_member_file_bodies": len(member_hash_occurrences),
        "repeated_member_byte_occurrences": file_occurrences - len(member_hash_occurrences),
        "member_bodies_referenced_more_than_once": sum(x > 1 for x in member_hash_occurrences.values()),
        "gg_binary_member_file_occurrences": suffix_counts[".gg"],
        "text_member_file_occurrences": suffix_counts[".txt"],
        "text_members_containing_credit_marker": credits_txt,
        "individual_member_names_codes_or_fingerprints_exported": False,
    }


def build():
    meta = json.loads((ROOT / "research/bulk-source-scout-v1.json").read_text("utf-8"))
    from audit_bulk_source_scout import verify
    verify(meta, ROOT)
    src = {x["id"]: x for x in meta["sources"]}
    def external(sid):
        item = json.loads((ROOT / "sources/external-bulk" / (sid + ".json")).read_text("utf-8"))
        if item["id"] != sid or item["rights_status"] != "link_only_rights_review":
            raise ValueError("Unexpected external rights or source identity")
        return item

    psp_a = external("saramagrean-psp-cwcheat")
    psp_b = external("apelfactory-ppsspp-cheatdb")
    psp_report = compare_cwcheat(
        read_verified(psp_a, psp_a["files"][0]),
        read_verified(psp_b, psp_b["files"][0]),
    )
    psp_report["a_source_id"] = psp_a["id"]
    psp_report["b_source_id"] = psp_b["id"]

    nintendo = src["riyoway-nintendo"]
    paths = {row[0]: row for row in nintendo["files"]}
    exports = {}
    for label in ("switch", "3ds", "nes", "wiiu"):
        path = f"exports/JSON/Nintendo/{label}.json"
        exports[label] = read_verified(nintendo, paths[path])
    nintendo_report = summarize_nintendo_exports(exports)
    prior = json.loads((ROOT / "research/bulk-source-format-audit-v1.json").read_text("utf-8"))
    sqlite = prior["nintendo_sqlite"]["selected_table_row_counts"]
    nintendo_reconciliation = {
        "canonical_sqlite_source_games": sqlite["games"],
        "canonical_sqlite_source_cheats": sqlite["cheats"],
        "export_game_records": sum(p["source_game_or_variant_records"] for p in nintendo_report.values()),
        "export_cheat_records": sum(p["cheat_record_occurrences"] for p in nintendo_report.values()),
    }
    nintendo_reconciliation["game_row_count_reconciled"] = (
        nintendo_reconciliation["canonical_sqlite_source_games"] ==
        nintendo_reconciliation["export_game_records"])
    nintendo_reconciliation["cheat_row_count_reconciled"] = (
        nintendo_reconciliation["canonical_sqlite_source_cheats"] ==
        nintendo_reconciliation["export_cheat_records"])
    if not (nintendo_reconciliation["game_row_count_reconciled"] and
            nintendo_reconciliation["cheat_row_count_reconciled"]):
        raise ValueError("Canonical SQLite and JSON exports disagree on source-declared counts")

    sega = src["slade1972-segacd"]
    zip_files = []
    for row in sega["files"]:
        zip_files.append((row[0], row[1], read_verified(sega, row)))
    sega_report = sega_zip_members(zip_files)

    return {
        "schema_version": 1,
        "format": FORMAT,
        "comparison": "pinned verified original source bytes, ephemeral exact syntax comparisons",
        "no_original_code_text_exported_or_committed": True,
        "no_identifiers_or_member_paths_exported": True,
        "no_per_code_fingerprints_exported": True,
        "rights_to_redistribute_established": False,
        "distinct_games_or_cheat_effects_verified": False,
        "gameplay_compatibility_verified": False,
        "notes": [
            "Nintendo JSON exports mirror canonical SQLite and are never summed as new cheat collections.",
            "PSP overlap means exact serial label and normalized _L instruction block equality, not verified code effects.",
            "Sega CD member-byte duplicates do not imply same game version or in-emulator cheat behavior.",
            "Attribution string presence does not establish redistribution permission.",
        ],
        "psp_cwcheat_pair": psp_report,
        "nintendo_source_exports_by_platform": nintendo_report,
        "nintendo_sqlite_export_reconciliation": nintendo_reconciliation,
        "segacd_member_byte_overlap": sega_report,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    report = build()
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True) + "\n", "utf-8")
    print("BEGIN_COMPILED_SOURCE_AUDIT")
    print(json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True))
    print("END_COMPILED_SOURCE_AUDIT")


if __name__ == "__main__":
    main()
