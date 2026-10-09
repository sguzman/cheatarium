#!/usr/bin/env python3
"""Archive CookiePLMonster's version-pinned original console code collection.

Retains all original files (including README, LICENSE, local credits and data)
and attributes 183 actual native cheat files to four consoles without changing
their names or bytes. Native cheat counts exclude upstream tooling/metadata.
"""
import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "https://github.com/CookiePLMonster/Console-Cheat-Codes"
COMMIT = "f55aa8db8e6c79a2af6a9dd216b46635fc7324b5"
SOURCE_ID = "cookieplmonster-console-cheat-codes"
ARCHIVE = ROOT / "archive" / SOURCE_ID
MANIFEST = ROOT / "sources" / f"{SOURCE_ID}.json"
CHEAT_RULES = {
    "PS1": (".cht", "ps1"),
    "PS2": (".pnach", "ps2"),
    "GC": (".ini", "gamecube"),
    "PSP": (".ini", "psp"),
}
EXPECTED = {"ps1": 67, "ps2": 110, "gamecube": 2, "psp": 4}
TOTAL_ORIGINAL_FILES = 204


def checked_sha(data):
    return hashlib.sha1(f"blob {len(data)}\0".encode("ascii") + data).hexdigest()


def tree_files(upstream):
    listed = subprocess.check_output(
        ["git", "-C", str(upstream), "ls-tree", "-r", "--name-only", "HEAD"],
        text=True,
    ).splitlines()
    if len(listed) != TOTAL_ORIGINAL_FILES or len(set(listed)) != len(listed):
        raise RuntimeError(f"Expected {TOTAL_ORIGINAL_FILES} unique original files; got {len(listed)}")
    for name in listed:
        path = PurePosixPath(name)
        if (path.is_absolute() or not path.parts or
                any(part in (".", "..", "") for part in path.parts)):
            raise RuntimeError(f"Unsafe upstream path {name!r}")
    return sorted(listed)


def classify(name):
    path = PurePosixPath(name)
    if not path.parts:
        return None
    rule = CHEAT_RULES.get(path.parts[0])
    if rule and path.suffix.lower() == rule[0]:
        return rule[1]
    return None


def import_pinned(upstream):
    actual = subprocess.check_output(
        ["git", "-C", str(upstream), "rev-parse", "HEAD"], text=True,
    ).strip()
    if actual != COMMIT:
        raise RuntimeError(f"Expected source revision {COMMIT}, got {actual}")
    files = tree_files(upstream)
    for required in ("LICENSE", "README.md"):
        if required not in files:
            raise RuntimeError(f"Upstream attribution notice absent: {required}")
    counts = {platform: 0 for platform in EXPECTED}
    old = {}
    if MANIFEST.exists():
        prior = json.loads(MANIFEST.read_text(encoding="utf-8"))
        if prior.get("snapshot_commit") != COMMIT or prior.get("id") != SOURCE_ID:
            raise RuntimeError("Refusing to merge different source revisions or IDs")
        old = {row["upstream_path"]: row for row in prior["files"]}
    inventory = []
    new_files = 0
    for rel in files:
        src = upstream / rel
        if not src.is_file() or src.is_symlink():
            raise RuntimeError(f"Nonregular source file {rel}")
        raw = src.read_bytes()
        if not raw:
            raise RuntimeError(f"Empty source file {rel}")
        record = {
            "upstream_path": rel,
            "archive_path": f"archive/{SOURCE_ID}/{rel}",
            "git_blob_sha": checked_sha(raw),
        }
        if rel in old and old[rel] != record:
            raise RuntimeError(f"Previously imported source changed: {rel}")
        dest = ARCHIVE / rel
        if dest.is_file():
            if dest.read_bytes() != raw:
                raise RuntimeError(f"Existing archived bytes conflict: {rel}")
        elif dest.exists():
            raise RuntimeError(f"Unexpected archive entry: {rel}")
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(raw)
            new_files += 1
        platform = classify(rel)
        if platform:
            counts[platform] += 1
        inventory.append(record)
    if counts != EXPECTED:
        raise RuntimeError(f"Native cheat count mismatch: {counts} != {EXPECTED}")
    if set(old).difference(files):
        raise RuntimeError("Pinned original inventory unexpectedly shrank")
    manifest = {
        "schema_version": 1,
        "id": SOURCE_ID,
        "repository": REPOSITORY,
        "snapshot_commit": COMMIT,
        "license": "MIT (root LICENSE); original/adapted code credits retained separately",
        "archive_prefix": f"archive/{SOURCE_ID}/",
        "scope_note": (
            "PS2 .pnach, PS1 .cht, GameCube GameINI .ini and PSP CWCheat .ini "
            "codes as originally supplied; original per-release and author text "
            "must not be treated as independently tested by Cheatarium."
        ),
        "rights_note": (
            "README explicitly states some imported codes belong to other authors "
            "and credits are given. Preserve each original code file, named "
            "credits.txt, readmes, scripts, root LICENSE and native layout. "
            "MIT root license does not silently override rights of other authors."
        ),
        "native_cheat_counts": counts,
        "files": inventory,
    }
    MANIFEST.write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({
        "revision": COMMIT,
        "new_original_files_including_notices": new_files,
        "original_cheat_files": sum(counts.values()),
        "native_cheat_counts": counts,
        "source_inventory_files": len(inventory),
    }, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checked-out-upstream", type=Path)
    args = parser.parse_args()
    if args.checked_out_upstream:
        import_pinned(args.checked_out_upstream)
        return
    with tempfile.TemporaryDirectory(prefix="cheatarium-ps2-") as temp:
        upstream = Path(temp) / "source"
        subprocess.run(
            ["git", "clone", "--quiet", "--filter=blob:none", "--depth", "1",
             "--no-checkout", f"{REPOSITORY}.git", str(upstream)], check=True,
        )
        subprocess.run(
            ["git", "-C", str(upstream), "fetch", "--quiet", "--depth", "1",
             "origin", COMMIT], check=True,
        )
        subprocess.run(
            ["git", "-C", str(upstream), "checkout", "--quiet", "--detach", COMMIT],
            check=True,
        )
        import_pinned(upstream)


if __name__ == "__main__":
    main()
