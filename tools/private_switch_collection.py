#!/usr/bin/env python3
"""Index, search, verify and back up locally acquired Switch cheats.

The source cache is authoritative. Eden installations are never read or changed.
Third-party code bodies and collection indexes stay outside the Git repository.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile

HERE = Path(__file__).resolve().parent.parent
MANIFEST = HERE / "research/switch-personal-curation-v1.json"
ROOT = Path.home() / ".local/share/cheatarium/private-switch-curation"
CONFIG = Path.home() / ".config/cheatarium/private-switch-collection.json"
TITLE = re.compile(r"^01[0-9A-F]{14}$")
BUILD = re.compile(r"^[0-9A-F]{16}$")
HEADING = re.compile(r"^\[([^\[\]\r\n]{1,128})\]$")
WORD = re.compile(r"^[0-9a-fA-F]{8}$")
MAX_SOURCE_BYTES = 1_000_000
MAX_RECEIPT_BYTES = 16_384


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def safe_directory(path: Path, create: bool = False) -> Path:
    if not path.is_absolute() or ".." in path.parts:
        raise ValueError("Directory must be normalized and absolute")
    current = Path(path.anchor)
    for part in path.parts[1:]:
        current = current / part
        if current.is_symlink():
            raise ValueError(f"Symlink in collection or backup path: {current}")
        if current.exists():
            if not current.is_dir():
                raise ValueError(f"Not a directory: {current}")
        elif create:
            current.mkdir(mode=0o700)
        else:
            raise ValueError(f"Directory is missing: {current}")
    return path


def checked_file(path: Path, limit: int) -> bytes:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise ValueError(f"Unsafe, missing or oversized private file: {path}")
    return path.read_bytes()


def parse_sections(body: bytes) -> list[dict]:
    text = body.decode("utf-8")
    sections: list[dict] = []
    name = None
    code: list[str] = []
    for line in (line.strip() for line in text.splitlines()):
        if not line:
            continue
        match = HEADING.fullmatch(line)
        if match:
            if name is not None:
                if not code:
                    raise ValueError("Empty native cheat section")
                sections.append({"name": name, "code_lines": len(code)})
            name, code = match.group(1), []
        else:
            if name is None:
                raise ValueError("Native code before a heading")
            words = line.split()
            if not (1 <= len(words) <= 5 and all(WORD.fullmatch(w) for w in words)):
                raise ValueError("Malformed native cheat line")
            code.append(line)
        if len(sections) > 2048:
            raise ValueError("Too many native cheat sections")
    if name is None or not code:
        raise ValueError("Empty or incomplete native cheat file")
    sections.append({"name": name, "code_lines": len(code)})
    return sections


def collect(root: Path, manifest_path: Path = MANIFEST) -> tuple[dict, list[Path]]:
    root = safe_directory(root)
    data = json.loads(checked_file(manifest_path, 2_000_000))
    if data.get("format") != "cheatarium-user-switch-cheat-source-curation-v1":
        raise ValueError("Unexpected collection source manifest")
    games = {game["title_id"]: game for game in data["games"]}
    if len(games) != len(data["games"]):
        raise ValueError("Duplicate Title ID in source manifest")
    records: list[dict] = []
    files: list[Path] = []
    for title_dir in sorted(root.iterdir()):
        if title_dir.name == "collection-index.json" and title_dir.is_file():
            if title_dir.is_symlink():
                raise ValueError("Symlink collection index")
            continue
        if not TITLE.fullmatch(title_dir.name) or title_dir.is_symlink() or not title_dir.is_dir():
            raise ValueError(f"Unexpected private collection entry: {title_dir.name}")
        game = games.get(title_dir.name)
        if game is None:
            raise ValueError(f"Unrecognized private game Title ID: {title_dir.name}")
        names = {p.name for p in title_dir.iterdir()}
        for filename in sorted(names):
            if filename.endswith(".source.json"):
                if filename.removesuffix(".source.json") + ".txt" not in names:
                    raise ValueError(f"Orphan provenance receipt: {filename}")
            elif not (filename.endswith(".txt") and BUILD.fullmatch(filename[:-4])):
                raise ValueError(f"Unexpected private cheat file: {filename}")
        for filename in sorted(n for n in names if n.endswith(".txt")):
            bid = filename[:-4]
            path = title_dir / filename
            body = checked_file(path, MAX_SOURCE_BYTES)
            sections = parse_sections(body)
            builds = [x for x in game["hamlet_builds"] if x["build_id"] == bid]
            if len(builds) > 1:
                raise ValueError(f"Duplicate pinned Build ID: {bid}")
            receipt_path = title_dir / (bid + ".source.json")
            if builds and builds[0]["strict_native_hex_sections"]:
                if len(sections) != builds[0]["strict_native_hex_sections"]:
                    raise ValueError(f"Pinned native section count mismatch: {filename}")
                if receipt_path.exists():
                    raise ValueError(f"Unexpected external receipt for pinned source: {filename}")
                provenance = {
                    "source_id": "hamlet-private",
                    "source_url": game["source_url"],
                    "revision": data["upstream"]["snapshot_commit"],
                    "verification": "pinned_source_lexical_only",
                }
            else:
                receipt = json.loads(checked_file(receipt_path, MAX_RECEIPT_BYTES))
                matching = [
                    s for s in game.get("exact_build_external_sources", [])
                    if s["source_id"] == receipt.get("source_id")
                    and s["source_url"] == receipt.get("source_url")
                    and s["build_id"] == bid
                ]
                if (len(matching) != 1 or
                        receipt.get("format") != "cheatarium-private-external-native-source-v1" or
                        receipt.get("title_id") != title_dir.name or
                        receipt.get("build_id") != bid or
                        receipt.get("body_sha256") != sha(body) or
                        receipt.get("section_count") != len(sections) or
                        receipt.get("gameplay_verified") is not False):
                    raise ValueError(f"Invalid external source receipt: {filename}")
                provenance = {
                    "source_id": receipt["source_id"],
                    "source_url": receipt["source_url"],
                    "revision": receipt["body_sha256"],
                    "verification": "receipt_sha256_lexical_only",
                }
                files.append(receipt_path.relative_to(root))
            files.append(path.relative_to(root))
            records.append({
                "title_id": title_dir.name,
                "game": game["title"],
                "build_id": bid,
                "relative_path": str(path.relative_to(root)),
                "bytes": len(body),
                "sha256": sha(body),
                "section_count": len(sections),
                "sections": [{"ordinal": i + 1, **section} for i, section in enumerate(sections)],
                "provenance": provenance,
                "gameplay_verified": False,
            })
    return {
        "format": "cheatarium-private-switch-collection-v1",
        "schema_version": 1,
        "authority": "local_private_original_sources_not_eden",
        "game_count": len({r["title_id"] for r in records}),
        "source_file_count": len(records),
        "section_count": sum(r["section_count"] for r in records),
        "records": records,
    }, files


def write_index(root: Path, index: dict) -> bytes:
    payload = (json.dumps(index, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    destination = root / "collection-index.json"
    if destination.exists() and checked_file(destination, 8_000_000) == payload:
        return payload
    fd, name = tempfile.mkstemp(prefix=".collection-index-", dir=root)
    try:
        with os.fdopen(fd, "wb") as out:
            os.fchmod(out.fileno(), 0o600)
            out.write(payload)
            out.flush()
            os.fsync(out.fileno())
        if destination.is_symlink():
            raise ValueError("Collection index destination is a symlink")
        os.replace(name, destination)
    finally:
        Path(name).unlink(missing_ok=True)
    return payload


def copy_verified(source: Path, dest: Path, *, verify_only: bool) -> None:
    safe_directory(dest.parent, create=not verify_only)
    data = checked_file(source, 8_000_000)
    if dest.exists() or dest.is_symlink():
        if checked_file(dest, 8_000_000) != data:
            raise ValueError(f"Backup conflict; refusing to overwrite: {dest}")
        return
    if verify_only:
        raise ValueError(f"Missing backup file: {dest}")
    fd = os.open(dest, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
    with os.fdopen(fd, "wb") as out:
        out.write(data)
        out.flush()
        os.fsync(out.fileno())


def back_up(root: Path, files: list[Path], index_payload: bytes, backup: Path, verify_only: bool = False) -> dict:
    if backup == root or backup in root.parents or root in backup.parents:
        raise ValueError("Backup must be independent of the source collection")
    if any((parent / ".git").exists() or (parent / ".git").is_symlink() for parent in (backup, *backup.parents)):
        raise ValueError("Refusing backup inside a Git working tree")
    if Path("/drive") in backup.parents and not os.path.ismount("/drive"):
        raise ValueError("Refusing /drive backup: /drive is not mounted")
    safe_directory(backup, create=not verify_only)
    for relative in files:
        copy_verified(root / relative, backup / relative, verify_only=verify_only)
    digest = sha(index_payload)
    snapshot = backup / "index-snapshots" / (digest + ".json")
    safe_directory(snapshot.parent, create=not verify_only)
    if snapshot.exists() or verify_only:
        if checked_file(snapshot, 8_000_000) != index_payload:
            raise ValueError("Collection index backup mismatch")
    else:
        fd = os.open(snapshot, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
        with os.fdopen(fd, "wb") as stream:
            stream.write(index_payload)
            stream.flush()
            os.fsync(stream.fileno())
    return {"status": "verified" if verify_only else "backed_up", "file_count": len(files), "index_sha256": digest, "backup_dir": str(backup)}


def saved_backup(config: Path = CONFIG) -> Path | None:
    if not config.exists():
        return None
    obj = json.loads(checked_file(config, 8192))
    if obj.get("format") != "cheatarium-private-collection-config-v1":
        raise ValueError("Unexpected private backup configuration")
    return Path(obj["backup_dir"])


def remember_backup(backup: Path, config: Path = CONFIG) -> None:
    safe_directory(config.parent, create=True)
    if config.is_symlink():
        raise ValueError("Backup configuration is a symlink")
    payload = (json.dumps({"format": "cheatarium-private-collection-config-v1", "backup_dir": str(backup)}, indent=2) + "\n").encode()
    fd, tmp = tempfile.mkstemp(prefix=".private-switch-backup-", dir=config.parent)
    try:
        with os.fdopen(fd, "wb") as out:
            os.fchmod(out.fileno(), 0o600)
            out.write(payload)
            out.flush()
            os.fsync(out.fileno())
        os.replace(tmp, config)
    finally:
        Path(tmp).unlink(missing_ok=True)


def refresh_collection(root: Path = ROOT, manifest: Path = MANIFEST, config: Path = CONFIG) -> dict:
    index, files = collect(root, manifest)
    payload = write_index(root, index)
    backup = saved_backup(config)
    status = back_up(root, files, payload, backup) if backup else {"status": "not_configured"}
    return {
        "games": index["game_count"],
        "source_files": index["source_file_count"],
        "sections": index["section_count"],
        "index_path": str(root / "collection-index.json"),
        "backup": status,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--manifest", type=Path, default=MANIFEST)
    parser.add_argument("--search", help="Find cached sections by name or game")
    parser.add_argument("--backup-to", type=Path, help="Back up raw private code and an immutable catalog snapshot")
    parser.add_argument("--remember-backup", action="store_true", help="Use --backup-to for later acquisitions")
    parser.add_argument("--verify-backup", action="store_true", help="Read-only verification of configured backup")
    args = parser.parse_args()
    if args.remember_backup and not args.backup_to:
        parser.error("--remember-backup requires --backup-to")
    if args.verify_backup and (args.backup_to or args.remember_backup):
        parser.error("--verify-backup cannot be combined with backup writes")
    index, files = collect(args.root, args.manifest)
    result = {
        "games": index["game_count"],
        "source_files": index["source_file_count"],
        "sections": index["section_count"],
    }
    if args.search:
        needle = args.search.casefold()
        result["matches"] = [
            {"game": r["game"], "title_id": r["title_id"], "build_id": r["build_id"], "name": s["name"], "ordinal": s["ordinal"]}
            for r in index["records"] for s in r["sections"]
            if needle in r["game"].casefold() or needle in s["name"].casefold()
        ]
    elif args.verify_backup:
        backup = saved_backup()
        if backup is None:
            raise ValueError("No configured backup destination")
        payload = (json.dumps(index, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
        result["backup"] = back_up(args.root, files, payload, backup, verify_only=True)
    else:
        payload = write_index(args.root, index)
        backup = args.backup_to or saved_backup()
        if backup is not None:
            result["backup"] = back_up(args.root, files, payload, backup)
            if args.remember_backup:
                remember_backup(backup)
        else:
            result["backup"] = {"status": "not_configured"}
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"Private Switch collection: {exc}", file=sys.stderr)
        sys.exit(1)
