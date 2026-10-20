#!/usr/bin/env python3
"""Preserve the six selected Switch games' actual native cheat bodies in Cheatarium.

Reads already acquired files, or obtains the same pinned upstream texts if absent.
No additional cache or Eden writes. --save writes only into this repository.
Publishing third-party authored bodies is a separate copyright/rights decision.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[1]
CACHE = Path.home() / ".local/share/cheatarium/private-switch-curation"
HEX = re.compile(r"^[0-9a-fA-F]{8}(?:\s+[0-9a-fA-F]{8}){0,4}$")
HEADER = re.compile(r"^\[([^\]\r\n]+)\]$")
EXPECTED_COUNT = 174
TITLES = {
    "0100000000010000": "super-mario-odyssey",
    "0100152000022000": "mario-kart-8-deluxe",
    "01002DA013484000": "the-legend-of-zelda-skyward-sword-hd",
    "01006BB00C6F0000": "the-legend-of-zelda-links-awakening",
    "0100BC0018138000": "super-mario-rpg",
    "01006A800016E000": "super-smash-bros-ultimate",
}
HAMLET_SOURCE = "switch-exact-hamlet"
SMASH_SOURCE = "switch-exact-smash"
MAX_BYTES = 1_000_000


def canonical(code: str) -> str:
    lines = []
    for raw in code.splitlines():
        words = raw.upper().split()
        if not words:
            continue
        if not 1 <= len(words) <= 5 or any(not re.fullmatch(r"[0-9A-F]{8}", w) for w in words):
            raise ValueError("Malformed native hexadecimal code")
        lines.append(" ".join(words))
    if not lines:
        raise ValueError("Empty native hexadecimal code")
    return "\n".join(lines)


def digest(code: str) -> str:
    return hashlib.sha256(canonical(code).encode("ascii")).hexdigest()


def native_sections(data: bytes) -> list[tuple[str, str]]:
    if len(data) > MAX_BYTES:
        raise ValueError("Oversized native cheat file")
    text = data.decode("utf-8-sig")
    entries: list[tuple[str, str]] = []
    heading = None
    body: list[str] = []
    def flush() -> None:
        if heading is not None:
            if not body:
                raise ValueError("Heading without native code: " + heading)
            entries.append((heading, "\n".join(body)))
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        match = HEADER.fullmatch(line)
        if match:
            flush()
            heading = "[" + match.group(1) + "]"
            body = []
        elif heading is None or not HEX.fullmatch(line):
            raise ValueError("Unexpected non-native source line")
        else:
            body.append(" ".join(line.upper().split()))
    flush()
    if len({name for name, _ in entries}) != len(entries):
        raise ValueError("Duplicate native heading")
    return entries


def source_bytes(game: dict, manifest: dict, cache: Path) -> tuple[bytes, str, str]:
    title_id = game["title_id"]
    bid = game["build_id"]
    original = cache / title_id / (bid + ".txt")
    if original.is_symlink() or original.parent.is_symlink():
        raise ValueError("Source cache contains symlink")
    if original.exists():
        if not original.is_file() or original.stat().st_size > MAX_BYTES:
            raise ValueError("Unexpected source-cache file")
        data = original.read_bytes()
        if title_id == "01006A800016E000":
            receipt = original.with_suffix(".source.json")
            if receipt.is_file():
                info = json.loads(receipt.read_text(encoding="utf-8"))
                if info["title_id"] != title_id or info["build_id"] != bid or hashlib.sha256(data).hexdigest() != info["body_sha256"]:
                    raise ValueError("Smash source receipt/checksum mismatch")
                return data, info["source_url"], info["source_id"]
        if title_id != "01006A800016E000":
            return data, game["source"]["url"], "pinned-hamlet-native-extract"
        # Unreceipted Smash body is still checked against the 25 pinned labels.
        return data, game["source"]["sources"][0]["url"], "attributed-smash-native"

    if title_id == "01006A800016E000":
        from fetch_smash_private import SOURCE, extract, fetch
        for source_id, url in SOURCE.items():
            try:
                return extract(fetch(source_id).decode("utf-8", errors="replace")), url, source_id
            except (ValueError, OSError, UnicodeError):
                continue
        raise ValueError("No complete exact-build Smash source obtainable")

    from curate_switch_private import parse_native
    url = game["source"]["url"]
    if not url.startswith("https://raw.githubusercontent.com/HamletDuFromage/switch-cheats-db/"):
        raise ValueError("Unexpected pinned source host")
    with urlopen(Request(url, headers={"User-Agent": "Cheatarium-Switch-Preservation/1"}), timeout=30) as response:
        if response.geturl() != url:
            raise ValueError("Unexpected redirect")
        body = response.read(MAX_BYTES + 1)
    if len(body) > MAX_BYTES:
        raise ValueError("Oversized upstream source")
    source = json.loads(body)
    if bid not in source:
        raise ValueError("Pinned Build ID not found")
    accepted, rejected = parse_native(source[bid])
    if rejected:
        # Only the explicit, exact-build accepted entries are preserved.
        expected = {row["heading"] for row in game["sections"]}
        if {name for name, _ in accepted} != expected:
            raise ValueError("Unexpected strict-native source exclusions")
    return "\n".join(code for _, code in accepted).encode("utf-8"), url, "pinned-hamlet-native-extract"


def ensure_no_symlinks(path: Path, root: Path) -> None:
    if not path.is_relative_to(root):
        raise ValueError("File outside Cheatarium")
    for parent in (path, *path.parents):
        if parent == root.parent:
            break
        if parent.is_symlink():
            raise ValueError("Symlink in destination: " + str(parent))


def source_manifest(source_id: str, rows: list[dict], upstream: dict) -> dict:
    is_hamlet = source_id == HAMLET_SOURCE
    return {
        "schema_version": 1,
        "id": source_id,
        "repository": upstream["repository"] if is_hamlet else rows[0]["source_url"],
        "snapshot_commit": upstream["snapshot_commit"] if is_hamlet else rows[0]["body_sha256"],
        "license": "unresolved-independent-contributor-rights",
        "rights_status": "redistribution_rights_unverified",
        "archive_prefix": "archive/" + source_id + "/",
        "source_format": "strict-native-text-extracted-from-pinned-source",
        "note": "Preserved local native-code extraction; file Git blob hashes identify these exact bytes and are not upstream JSON Git blob hashes.",
        "files": [{
            "upstream_path": row["title_id"] + "/" + row["build_id"] + ".txt",
            "archive_path": row["archive_path"],
            "git_blob_sha": row["git_blob_sha"],
            "source_url": row["source_url"],
            "source_label": row["source_label"],
            "source_body_sha256": row["body_sha256"],
            "native_section_count": row["section_count"],
        } for row in rows],
    }


def git_blob_sha(raw: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(raw)).encode("ascii") + b"\0" + raw).hexdigest()


def prepare(root: Path, cache: Path) -> tuple[dict[str, bytes], dict]:
    report = json.loads((root / "research/switch-exact-build-coverage-v1.json").read_text("utf-8"))
    manifest = json.loads((root / "research/switch-personal-curation-v1.json").read_text("utf-8"))
    by_title = {row["title_id"]: row for row in manifest["games"]}
    if report["counts"]["source_sections"] != EXPECTED_COUNT or len(report["games"]) != 6:
        raise ValueError("Pinned six-game manifest count differs")

    output: dict[str, bytes] = {}
    source_rows: dict[str, list[dict]] = {HAMLET_SOURCE: [], SMASH_SOURCE: []}
    games = 0
    sections_total = 0
    newly_curated = 0

    for game in report["games"]:
        title, bid = game["title_id"], game["build_id"]
        if title not in TITLES or by_title[title]["static_nsp_observation"]["cheat_build_id"] != bid:
            raise ValueError("Unrecognized or mismatched game Build ID")
        expected_rows = game["sections"]
        data, url, label = source_bytes(game, manifest, cache)
        entries = native_sections(data)
        if len(entries) != game["section_count"]:
            raise ValueError("Exact-build section count mismatch: " + title)
        if len(expected_rows) != len(entries):
            raise ValueError("Pinned source heading count mismatch")
        for row, (name, body) in zip(expected_rows, entries):
            if name.casefold() != row["heading"].casefold():
                raise ValueError("Source cheat heading changed: " + title)
            if row.get("source_code_sha256") and digest(body) != row["source_code_sha256"]:
                raise ValueError("Pinned code-body checksum mismatch: " + title + " " + name)

        source_id = SMASH_SOURCE if title == "01006A800016E000" else HAMLET_SOURCE
        archive_path = "archive/" + source_id + "/" + title + "/" + bid + ".txt"
        output[archive_path] = data
        source_rows[source_id].append({
            "title_id": title, "build_id": bid, "source_url": url,
            "source_label": label, "archive_path": archive_path,
            "git_blob_sha": git_blob_sha(data),
            "body_sha256": hashlib.sha256(data).hexdigest(),
            "section_count": len(entries),
        })

        curated_path = game["curated_file"]
        p = root / curated_path
        if p.is_file():
            curated = json.loads(p.read_text("utf-8"))
            if curated["game"]["title_id"] != title:
                raise ValueError("Curated Title ID mismatch")
        else:
            curated = {
                "schema_version": 1, "platform": "switch",
                "game": {"id": TITLES[title], "title": game["title"], "title_id": title},
                "editions": [], "cheats": [],
            }
        edition = "build-" + bid.lower()
        matched = [x for x in curated["editions"] if x["id"] == edition]
        if matched and matched[0]["build_id"] != bid:
            raise ValueError("Curated Build ID conflict")
        if not matched:
            curated["editions"].append({
                "id": edition, "region": "Unspecified", "revision": None,
                "hashes": {}, "build_id": bid
            })
        for row, (heading, code) in zip(expected_rows, entries):
            canonical_body = canonical(code)
            existing = [
                cheat for cheat in curated["cheats"]
                for variant in cheat["variants"]
                if variant["edition"] == edition and digest(variant["code"]) == digest(canonical_body)
            ]
            if existing:
                continue
            original_name = heading[1:-1].removeprefix("♯").strip()
            cheat_id = "exact-" + bid.lower() + "-" + str(row["ordinal"]).zfill(3)
            if any(c["id"] == cheat_id for c in curated["cheats"]):
                raise ValueError("Curated cheat ID conflict")
            curated["cheats"].append({
                "id": cheat_id, "name": original_name, "kind": "patch",
                "notes": "Source-reported native cheat; build match and gameplay not independently verified.",
                "variants": [{
                    "edition": edition, "format": "atmosphere-native-switch",
                    "code": canonical_body, "status": "unverified",
                    "source": {"source": source_id, "archive_path": archive_path},
                }]
            })
            newly_curated += 1
        output[curated_path] = (json.dumps(curated, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        games += 1
        sections_total += len(entries)

    if games != 6 or sections_total != EXPECTED_COUNT:
        raise ValueError("Six-game preservation incomplete")
    for source_id, rows in source_rows.items():
        file = "sources/" + source_id + ".json"
        source = source_manifest(source_id, rows, manifest["upstream"])
        output[file] = (json.dumps(source, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    return output, {"games": games, "sections": sections_total, "curated_new": newly_curated,
                    "archives": sum(map(len, source_rows.values()))}


def save_files(root: Path, output: dict[str, bytes]) -> list[str]:
    changes = []
    for relative, raw in output.items():
        path = root / relative
        ensure_no_symlinks(path, root)
        if relative.startswith("archive/") and path.exists() and path.read_bytes() != raw:
            raise ValueError("Refusing to alter an existing original cheat file: " + relative)
    for relative, raw in output.items():
        path = root / relative
        if path.is_file() and path.read_bytes() == raw:
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(raw)
        changes.append(relative)
    return changes


def verify(root: Path, cache: Path) -> dict:
    output, result = prepare(root, cache)
    missing = []
    for relative, data in output.items():
        if not (root / relative).is_file() or (root / relative).read_bytes() != data:
            missing.append(relative)
    if missing:
        raise ValueError("Missing or changed preserved files: " + ", ".join(missing))
    return {"status": "preserved_in_cheatarium", **result, "all_files_match": True}


def save_git(root: Path, touched: list[str]) -> None:
    if not touched:
        return
    def git(*args: str) -> str:
        return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()
    if git("branch", "--show-current") != "main" or Path(git("rev-parse", "--show-toplevel")).resolve() != root.resolve():
        raise ValueError("Expected existing Cheatarium checkout on main")
    remote = git("remote", "get-url", "origin")
    if not (remote.endswith("sguzman/cheatarium.git") or remote.endswith("sguzman/cheatarium")):
        raise ValueError("Unexpected Git origin; refusing to publish")
    # Refuse to include the user's unrelated staged changes.
    git("add", "--", *touched)
    subprocess.run(["git", "-C", str(root), "commit", "--only",
                    "-m", "data: preserve six exact-build Switch cheat collections",
                    "--", *touched], check=True)
    subprocess.run(["git", "-C", str(root), "push", "origin", "HEAD:main"], check=True)


def main() -> int:
    cli = argparse.ArgumentParser(description=__doc__)
    mode = cli.add_mutually_exclusive_group(required=True)
    mode.add_argument("--save", action="store_true", help="Write complete sources and curated codes into the checked-out Cheatarium repository")
    mode.add_argument("--verify", action="store_true", help="Read-only complete-preservation verification")
    cli.add_argument("--publish", action="store_true", help="After --save, commit only imported files on main and push to origin")
    cli.add_argument("--cache", type=Path, default=CACHE, help="Existing local source cache if available")
    args = cli.parse_args()
    if args.publish and not args.save:
        cli.error("--publish requires --save")
    if args.save:
        output, summary = prepare(ROOT, args.cache)
        changed = save_files(ROOT, output)
        # Update the already existing exact-build source coverage report from the repository's actual bytes.
        from reconcile_switch_exact_build_coverage import reconcile
        coverage_path = ROOT / "research/switch-exact-build-coverage-v1.json"
        report = reconcile(ROOT, json.loads(coverage_path.read_text("utf-8")))
        if report["counts"]["archived_exact_sections"] != 174 or report["counts"]["linked_only_sections"] != 0:
            raise ValueError("Preservation coverage is not complete: " + str(report["counts"]))
        payload = (json.dumps(report, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        if coverage_path.read_bytes() != payload:
            coverage_path.write_bytes(payload)
            changed.append(str(coverage_path.relative_to(ROOT)))
        if args.publish:
            save_git(ROOT, changed)
        print(json.dumps({"status": "saved", "changed_files": changed,
                          "published": args.publish, **summary}, indent=2))
    else:
        print(json.dumps(verify(ROOT, args.cache), indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError, subprocess.CalledProcessError) as exc:
        print(f"Switch collection preservation failed: {exc}", file=sys.stderr)
        sys.exit(1)
