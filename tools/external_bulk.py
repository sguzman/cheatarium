#!/usr/bin/env python3
"""Reproducible metadata-only inventory for rights-unresolved bulk cheat sources.

The pinned references expose names, Git blob IDs, and byte sizes. This tool
never copies third-party cheat or patch bodies into the Cheatarium archive.
"""
import argparse
import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FOLDER = Path("sources/external-bulk")
SOURCES = {
    "duckstation-chtdb": {
        "repository": "duckstation/chtdb",
        "commit": "e3cce509936261808b971199be3a20e84d0b9482",
        "platform": "ps1", "expected": {"cheat": 4497, "patch": 138},
        "rights_note": "The upstream LICENSE explicitly reserves ownership of cheats and patches to original authors; only processing scripts receive its MIT grant. File paths, hashes and byte sizes are indexed here, not code or patch bodies.",
    },
    "xenia-canary-game-patches": {
        "repository": "xenia-canary/game-patches",
        "commit": "106bb0a40be27b5a2444162562975e4885e1ce74",
        "platform": "xbox-360", "expected": {"patch": 501},
        "rights_note": "No repository-wide LICENSE was present in the pinned tree; patch authorship and redistribution permissions must be reviewed. Patches are not necessarily gameplay cheats. Only file names, hashes and sizes are indexed.",
    },
    "pcsx2-official-patches": {
        "repository": "PCSX2/pcsx2_patches",
        "commit": "9f82a4d2b8a2aaf83807421f18d5d30263ea826c",
        "platform": "ps2", "expected": {"patch":4710},
        "rights_note": "The official PCSX2 repository aggregates patches written by many independent authors, and no blanket permission for their code was located at this pinned revision. Native PNACH patch files are link-only references, not imported.",
    },
    "r0ah-vitacheat": {
        "repository": "r0ah/vitacheat",
        "commit": "bb8158a1c696914a8ea2299889d42ab9a57a3ab2",
        "platform": "vita", "expected": {"cheat":677},
        "rights_note": "Community-submitted VitaCheat code authors retain separate rights; no root redistribution LICENSE was found at the pinned revision. This inventory includes 676 standard PSV files and one specially named PSV-NSFW variant; no payload is rehosted.",
    },
    "shumnot-vitacheat": {
        "repository": "ShumnoT/VitaCheatDatabase",
        "commit": "a394316be0fe7b233b49b7ba22f14fae56a1b9c8",
        "platform": "vita", "expected": {"cheat":119},
        "rights_note": "The database credits multiple earlier websites and authors and lacks a blanket source-code redistribution grant. Preserve only original filename, Git blob hash and byte count.",
    },
    "bucanero-dreamcast-cheats": {
        "repository": "bucanero/dreamcast-cheats",
        "commit": "13c7a5c9957331398252e2a8916be7dd61b68144",
        "platform": "dreamcast", "expected": {"cheat":387},
        "rights_note": "The GPL-3.0 repository-wide grant does not independently establish redistribution rights for third-party historical cheats copied or adapted from external authors. The original markdown cheat text remains link-only.",
    },
    "ademola-switch-native": {
        "repository": "ADEMOLA200/Switch-Emulator-Mod-Database",
        "commit": "abd55774c369b9c3a4df960e7afdb0391cb52056",
        "platform": "switch", "expected": {"cheat":1927,"patch":589},
        "rights_note": "The MIT-licensed upstream aggregates many separately authored Switch cheat/graphics mods and patch sources; original contributors' redistribution rights are unresolved. Links, paths and hashes only; FPS/resolution patches are not necessarily cheats.",
    },
    "rogamer97-asm-3ds": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "3ds", "expected": {"cheat":53},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-ds": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "ds", "expected": {"cheat":5},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-gamecube": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "gamecube", "expected": {"cheat":28,"patch":2},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-wii": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "wii", "expected": {"cheat":305,"patch":1},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "rogamer97-asm-switch": {
        "repository": "RoGamer97/ASM-Cheats-and-Patches",
        "commit": "8ec531942871cd4e0aaefe5eb596fb2b49e9ae92",
        "platform": "switch", "expected": {"cheat":2,"patch":416},
        "rights_note": "No root repository LICENSE at pinned revision. Markdown/ASM/PCHTXT/IPS material has separate original authors; only links, original paths, blob hashes and sizes are indexed. No code bodies copied.",
    },
    "dospamu-rpcs3-artemis-patches": {
        "repository": "DoSpamu/RPCS3toArtemisPatches",
        "commit": "193ad2fc8001296796eb951f054efd855631ba47",
        "platform": "ps3", "expected": {"patch":2687},
        "rights_note": "MIT root repository contains RPCS3 and PSXPlace derived community patches by separate credited authors; publisher's MIT notice alone does not establish their redistribution rights. Original Artemis NCL conversions are link-only, may overlap existing Artemis codes, and upstream 'confirmed' labels are unverified by Cheatarium.",
    },
    "chidreams-rpcs3-patches": {
        "repository": "chidreams/Artemis-Patch-Collection-RPCS3",
        "commit": "c71ea69d28dc04ae8c7bc85cd513a8fa08909ad2",
        "platform": "ps3", "expected": {"patch":239},
        "rights_note": "MIT upstream collates historically sourced Artemis/RPCS3 patches from community authors; original-code rights are not independently confirmed. The 239 root-level named patch blobs have no file extensions; they are links only, not verified cheat effects.",
    },
    "apelfactory-ppsspp-cheatdb": {
        "repository": "ApelFactory/ppsspp-psp-cheatdb",
        "commit": "a0caf4c7e151e813781730046e46dabd815294e3",
        "platform": "psp", "expected": {"cheat":1},
        "rights_note": "The 8.7 MB PSP CWCheat cheat.db database has no root redistribution LICENSE at the pinned snapshot and contains historically compiled third-party cheat codes. Retain only its filename, byte size, original Git blob identity and pinned upstream URL.",
    },
    "real-f00-usrcheat-ds": {
        "repository": "real-F-00/usrcheat-mkds",
        "commit": "c0f4b13d9982add3e155ba99cd9cbeb9ab933f59",
        "platform": "ds", "expected": {"cheat":1},
        "rights_note": "The repository's Unlicense cannot by itself establish redistribution permission for separately authored cheat codes within its 42.98 MB binary usrcheat.dat DS database. No code, binary cheat payload, or game identities have been copied.",
    },
    "hamlet-switch-cheatdb": {
        "repository": "HamletDuFromage/switch-cheats-db",
        "commit": "9a062e9b9a9e5b422d0310cf5c829a3855cb77e7",
        "platform": "switch", "expected": {"cheat":4746, "patch":532},
        "rights_note": "The upstream README explicitly describes mirrors of the GBAtemp Switch cheat collection and ChanseyIsTheBest graphics/FPS cheat database, credits independent cheat authors, and provides no root redistribution LICENSE. Preserve only original JSON filenames, Git blob identities, sizes and links. Cheat/gfx mirrors may repeat content and are not independently verified game titles, cheat effects, or compatible builds.",
    },
}
SHA = re.compile(r"^[0-9a-f]{40}$")


def classify(source, path):
    if source == "duckstation-chtdb":
        if re.fullmatch(r"cheats/[^/]+\.cht", path):
            return "cheat"
        if re.fullmatch(r"patches/[^/]+\.cht", path):
            return "patch"
    if source == "xenia-canary-game-patches" and re.fullmatch(r"patches/[^/]+\.patch\.toml", path):
        return "patch"
    if source == "pcsx2-official-patches" and re.fullmatch(r"patches/[^/]+\.pnach", path):
        return "patch"
    if source == "r0ah-vitacheat" and re.fullmatch(r"db/[^/]+\.psv(?:-NSFW)?", path):
        return "cheat"
    if source == "shumnot-vitacheat" and re.fullmatch(r"[^/]+\.psv", path):
        return "cheat"
    if source == "bucanero-dreamcast-cheats" and re.fullmatch(r"[^/]+\.md", path) and path != "README.md":
        return "cheat"
    if source == "ademola-switch-native":
        if (re.fullmatch(r"Titles/[0-9a-fA-F]{16}/cheats/[0-9a-fA-F]{16}\.txt", path)
                or re.fullmatch(r"NX-60FPS-RES-GFX-Cheats/titles/[0-9a-fA-F]{16}/cheats/[0-9a-fA-F]{16}\.txt", path)):
            return "cheat"
        if path.endswith(".pchtxt"):
            return "patch"
    if source.startswith("rogamer97-asm-"):
        prefix = {"3ds": "3DS", "ds": "DS", "gamecube": "GameCube",
                  "switch": "Switch", "wii": "Wii"}.get(source.removeprefix("rogamer97-asm-"))
        if prefix and path.startswith(prefix + "/"):
            if path.endswith(".md") and path.rsplit("/", 1)[-1].lower() != "readme.md":
                return "cheat"
            if path.endswith((".pchtxt", ".ips", ".s")):
                return "patch"
    if source == "dospamu-rpcs3-artemis-patches" and re.fullmatch(
            r"(?:USERLIST|PSXPlace Confirmed|Working Artemis Patches)/[^/]+\.ncl", path):
        return "patch"
    if source == "chidreams-rpcs3-patches" and path and not path.startswith(".") and "/" not in path:
        return "patch"
    if source == "apelfactory-ppsspp-cheatdb" and path == "cheat.db":
        return "cheat"
    if source == "real-f00-usrcheat-ds" and path == "usrcheat.dat":
        return "cheat"
    if source == "hamlet-switch-cheatdb":
        if re.fullmatch(r"(?:cheats|cheats_gbatemp)/[0-9a-fA-F]{16}\.json", path):
            return "cheat"
        if re.fullmatch(r"cheats_gfx/[0-9a-fA-F]{16}\.json", path):
            return "patch"
    return None


def make_index(source, tree):
    spec = SOURCES[source]
    if tree.get("truncated") or not isinstance(tree.get("tree"), list):
        raise ValueError("Git tree incomplete")
    rows = sorted(
        [[item["path"], item["sha"], item["size"]]
         for item in tree["tree"]
         if item.get("type") == "blob" and classify(source, item.get("path", ""))],
        key=lambda row: row[0],
    )
    return {
        "schema_version": 1, "id": source,
        "repository": "https://github.com/" + spec["repository"],
        "snapshot_commit": spec["commit"],
        "git_tree_sha": tree["sha"],
        "platform": spec["platform"],
        "rights_status": "link_only_rights_review",
        "rights_note": spec["rights_note"],
        "filename_scope": "source-native cheat and patch files only; source contents intentionally omitted",
        "file_counts": count_types(source, rows),
        "files": rows,
    }


def count_types(source, rows):
    counts = {}
    for path, _, _ in rows:
        kind = classify(source, path)
        if kind is None:
            raise ValueError("Invalid source-native path: " + path)
        counts[kind] = counts.get(kind, 0) + 1
    return counts


def validate(source, index):
    spec = SOURCES[source]
    if (index.get("schema_version") != 1 or index.get("id") != source
            or index.get("repository") != "https://github.com/" + spec["repository"]
            or index.get("snapshot_commit") != spec["commit"]
            or index.get("platform") != spec["platform"]
            or index.get("rights_status") != "link_only_rights_review"
            or index.get("rights_note") != spec["rights_note"]
            or index.get("filename_scope") != "source-native cheat and patch files only; source contents intentionally omitted"
            or not SHA.fullmatch(index.get("git_tree_sha", ""))):
        raise ValueError(f"Metadata or rights status mismatch for {source}")
    rows = index.get("files")
    if not isinstance(rows, list):
        raise ValueError("Missing source file inventory")
    previous = ""
    for row in rows:
        if not (isinstance(row, list) and len(row) == 3):
            raise ValueError("Unexpected field in a metadata-only file record")
        path, blob, size = row
        if (not isinstance(path, str) or "\x00" in path
                or path.startswith("/") or "\\" in path
                or any(seg in ("", ".", "..") for seg in path.split("/"))
                or path <= previous or classify(source, path) is None
                or not isinstance(blob, str) or not SHA.fullmatch(blob)
                or not isinstance(size, int) or isinstance(size, bool) or size < 0):
            raise ValueError(f"Unsafe or invalid record: {row!r}")
        previous = path
    if count_types(source, rows) != spec["expected"] or index.get("file_counts") != spec["expected"]:
        raise ValueError("Pinned source counts changed for " + source)
    if set(index) != {"schema_version", "id", "repository", "snapshot_commit",
                      "git_tree_sha", "platform", "rights_status", "rights_note",
                      "filename_scope", "file_counts", "files"}:
        raise ValueError("Unexpected manifest fields may indicate code payload leakage")


def source_link(index, path):
    encoded = urllib.parse.quote(path, safe="/")
    return index["repository"] + "/blob/" + index["snapshot_commit"] + "/" + encoded


def lookup(indexes, needle, platform=None, limit=20):
    needle = needle.casefold()
    found = []
    for index in indexes:
        if platform and platform != index["platform"]:
            continue
        for path, blob, size in index["files"]:
            if needle not in path.casefold():
                continue
            found.append({"platform": index["platform"], "source": index["id"],
                          "kind": classify(index["id"], path), "path": path,
                          "git_blob_sha": blob, "bytes": size,
                          "upstream_url": source_link(index, path),
                          "redistribution": "link_only_rights_review"})
            if len(found) >= limit:
                return found
    return found


def load(root):
    result = []
    for source in SOURCES:
        doc = json.loads((root / FOLDER / (source + ".json")).read_text("utf-8"))
        validate(source, doc)
        result.append(doc)
    return result


def refresh(root):
    for source, spec in SOURCES.items():
        url = "https://api.github.com/repos/" + spec["repository"] + "/git/trees/" + spec["commit"] + "?recursive=1"
        request = urllib.request.Request(url, headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "Cheatarium-external-bulk-metadata-index",
        })
        with urllib.request.urlopen(request, timeout=60) as response:
            upstream = json.load(response)
        index = make_index(source, upstream)
        validate(source, index)
        dest = root / FOLDER / (source + ".json")
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(json.dumps(index, indent=2, ensure_ascii=False) + "\n", "utf-8")
        print(f"Refreshed link-only source {source}: {len(index['files'])} filename/hash records")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--refresh", action="store_true")
    mode.add_argument("--search", metavar="TEXT")
    parser.add_argument("--platform", choices=("ps1", "xbox-360", "ps2", "vita", "dreamcast", "switch", "3ds", "ds", "gamecube", "wii", "ps3", "psp"))
    parser.add_argument("--limit", type=int, default=20)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    if args.limit < 1 or args.limit > 100:
        parser.error("--limit must be in [1, 100]")
    if args.refresh:
        refresh(args.root)
    else:
        indexes = load(args.root)
        if args.check:
            for doc in indexes:
                print(f"VALID metadata only: {doc['id']} {len(doc['files'])} original upstream paths")
        else:
            print(json.dumps({"query": args.search, "results": lookup(indexes, args.search, args.platform, args.limit),
                              "note": "External source links, not archived or independently gameplay-verified codes."}, indent=2))


if __name__ == "__main__":
    main()
