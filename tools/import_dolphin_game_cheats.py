#!/usr/bin/env python3
"""Preserve the pinned Dolphin GameSettings containing real AR/Gecko cheat codes.

The GameSettings directory also contains many ordinary emulator preferences.
Select only native INI files with a named code and a substantive code line in
an [ActionReplay] or [Gecko] section; preserve each selected file verbatim.
No ROM/build matching or claims about original cheat authors' rights.
"""
import hashlib
import json
import re
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
REPO = "https://github.com/dolphin-emu/dolphin"
COMMIT = "e6f3ae17627e4344da95b13424af5baf4c892b08"
SOURCE_ID = "dolphin-game-cheats"
SOURCE_DIR = "Data/Sys/GameSettings/"
MANIFEST = ROOT / "sources" / f"{SOURCE_ID}.json"
ARCHIVE = ROOT / "archive" / SOURCE_ID
EXPECTED_INIS = 1889
NOTICES = (
    "COPYING",
    "Readme.md",
    "LICENSES/GPL-2.0-or-later.txt",
    "LICENSES/MIT.txt",
    "LICENSES/BSD-3-Clause.txt",
    "LICENSES/CC0-1.0.txt",
)
HEADING = re.compile(r"^\$[^\r\n]+$")
PAYLOAD = re.compile(r"^[A-Z0-9]{8}\s+[A-Z0-9]{8}(?:\s|$)", re.I)
SECTION = re.compile(r"^\[([^\]]+)\]$")


def git_blob(raw):
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def cheat_section_count(raw):
    text = raw.decode("utf-8", errors="replace")
    active = False
    seen_heading = False
    with_payload = 0
    for line in text.splitlines():
        line = line.strip()
        marker = SECTION.fullmatch(line)
        if marker:
            active = marker.group(1) in ("ActionReplay", "Gecko")
            seen_heading = False
            continue
        if not active or line.startswith("#") or not line:
            continue
        if HEADING.fullmatch(line):
            seen_heading = True
        elif seen_heading and PAYLOAD.match(line):
            with_payload += 1
            seen_heading = False
    return with_payload


def classify_name(file):
    # This is only a source-disc-system hint, never verified build identity.
    # Other one-character Dolphin GameSettings keys are archived as unresolved.
    stem = file.rsplit("/", 1)[-1][:-4]
    if len(stem) != 6:
        return "unclassified"
    if stem[0] in "GD":
        return "gamecube"
    if stem[0] in "RSWH":
        return "wii"
    return "unclassified"


def import_snapshot(checkout):
    sha = subprocess.check_output(
        ["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True
    ).strip()
    if sha != COMMIT:
        raise RuntimeError(f"Unexpected upstream revision: {sha}")
    listing = sorted(subprocess.check_output(
        ["git", "-C", str(checkout), "ls-tree", "-r", "--name-only", "HEAD",
         SOURCE_DIR, "COPYING", "Readme.md", "LICENSES"],
        text=True
    ).splitlines())
    inis = [path for path in listing
            if path.startswith(SOURCE_DIR) and path.endswith(".ini")]
    if len(inis) != EXPECTED_INIS or len(set(inis)) != EXPECTED_INIS:
        raise RuntimeError(f"Incomplete Dolphin GameSettings tree: {len(inis)} INI files")
    if not set(NOTICES).issubset(set(listing)):
        raise RuntimeError("License or attribution notice missing from pinned upstream")
    selected = set(NOTICES)
    counts = {"gamecube": 0, "wii": 0, "unclassified": 0}
    sections = 0
    for name in inis:
        src = checkout / name
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Nonregular upstream INI: {name}")
        found = cheat_section_count(src.read_bytes())
        if not found:
            continue
        counts[classify_name(name)] += 1
        sections += found
        selected.add(name)
    if sum(counts.values()) < 25 or sections < sum(counts.values()):
        raise RuntimeError(f"Suspiciously few original AR/Gecko cheat files: {counts}")
    old = {}
    if MANIFEST.is_file():
        previous = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if previous.get("id") != SOURCE_ID or previous.get("snapshot_commit") != COMMIT:
            raise RuntimeError("Cannot merge different upstream source snapshots")
        old = {e["upstream_path"]: e for e in previous["files"]}
    inventory = []
    added = 0
    for name in sorted(selected):
        rel = PurePosixPath(name)
        if rel.is_absolute() or not rel.parts or any(p in ("", ".", "..") for p in rel.parts):
            raise RuntimeError(f"Unsafe upstream path: {name}")
        src = checkout / name
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Nonregular source file: {name}")
        raw = src.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty upstream file: {name}")
        record = {
            "upstream_path": name,
            "archive_path": f"archive/{SOURCE_ID}/{name}",
            "git_blob_sha": git_blob(raw),
        }
        if name in old and old[name] != record:
            raise RuntimeError(f"Original upstream bytes changed: {name}")
        dest = ARCHIVE / name
        if dest.exists():
            if not dest.is_file() or dest.read_bytes() != raw:
                raise RuntimeError(f"Archived bytes differ from source: {name}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(raw)
            added += 1
        inventory.append(record)
    if set(old).difference(selected):
        raise RuntimeError("Cannot remove previously archived original files")
    manifest = {
        "schema_version": 1,
        "id": SOURCE_ID,
        "repository": REPO,
        "snapshot_commit": COMMIT,
        "license": "GPL-2.0-or-later project-wide guidance; third-party per-file rights may vary",
        "archive_prefix": f"archive/{SOURCE_ID}/",
        "scope_note": "Selected upstream Dolphin GameSettings INI source files with explicit ActionReplay/Gecko named code payloads. Excludes ordinary emulator settings INIs.",
        "rights_note": "Dolphin COPYING explains GPLv2+ for most original work and differing licenses for derived materials. Preserve root Readme.md, COPYING, LICENSES notices and original file-level credits; do not assume individual authors surrendered separate rights.",
        "selection_rule": "An INI is included if an ActionReplay or Gecko section contains a $-prefixed named entry followed by an original 8+8-character code line. Other INIs omitted.",
        "source_game_id_hints_only": True,
        "native_cheat_file_counts": counts,
        "named_payload_sections_detected": sections,
        "files": inventory,
    }
    MANIFEST.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(json.dumps({
        "source_revision": COMMIT,
        "new_files_including_notices": added,
        "selected_original_cheat_ini_files": sum(counts.values()),
        "gamecube_wii_unresolved": counts,
        "named_payload_sections_detected": sections,
        "total_inventory_files": len(inventory),
    }, indent=2))


def main():
    with tempfile.TemporaryDirectory(prefix="cheatarium-dolphin-") as tmp:
        checkout = Path(tmp) / "upstream"
        subprocess.run([
            "git", "clone", "--quiet", "--filter=blob:none", "--depth", "1",
            "--sparse", "--no-checkout", REPO + ".git", str(checkout)
        ], check=True)
        subprocess.run([
            "git", "-C", str(checkout), "fetch", "--quiet", "--depth", "1",
            "origin", COMMIT
        ], check=True)
        subprocess.run([
            "git", "-C", str(checkout), "checkout", "--quiet", "--detach", COMMIT
        ], check=True)
        subprocess.run([
            "git", "-C", str(checkout), "sparse-checkout", "set",
            "Data/Sys/GameSettings", "LICENSES"
        ], check=True)
        import_snapshot(checkout)


if __name__ == "__main__":
    main()
