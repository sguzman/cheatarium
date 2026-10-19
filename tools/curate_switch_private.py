#!/usr/bin/env python3
"""Private, source-pinned Switch cheat curation. Never writes into Eden.

Public repo carries only source metadata and audit logic. Third-party code
is fetched on user request and stays under ~/.local/share/cheatarium/.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from urllib.request import Request, urlopen

MANIFEST = Path(__file__).resolve().parent.parent / "research/switch-personal-curation-v1.json"
BUILD_ID = re.compile(r"^[0-9A-F]{16}$")
HEX_WORD = re.compile(r"^[0-9A-Fa-f]{8}$")
LIMIT = 1_000_000


def parse_native(build_data: object) -> tuple[list[tuple[str, str]], list[str]]:
    """Preserve original named hexadecimal sections; reject malformed ones."""
    if not isinstance(build_data, dict):
        raise ValueError("Build record is not a dictionary")
    accepted: list[tuple[str, str]] = []
    rejected: list[str] = []
    for name, code in build_data.items():
        if not isinstance(name, str) or not isinstance(code, str):
            rejected.append(str(name)[:90])
            continue
        lines = [line.strip() for line in code.splitlines() if line.strip()]
        if (
            not lines
            or lines[0] != name
            or not name.startswith("[")
            or not name.endswith("]")
            or any(c in name for c in "\r\n\x00")
            or len(name) > 130
        ):
            rejected.append(name)
            continue
        if not lines[1:] or not all(
            1 <= len(parts := line.split()) <= 5
            and all(HEX_WORD.fullmatch(p) for p in parts)
            for line in lines[1:]
        ):
            rejected.append(name)
            continue
        accepted.append((name, "\n".join(lines) + "\n"))
    return accepted, rejected


def load_manifest() -> dict:
    obj = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if obj["format"] != "cheatarium-user-switch-cheat-source-curation-v1":
        raise ValueError("Unexpected curated source manifest")
    if not re.fullmatch(r"[0-9a-f]{40}", obj["upstream"]["snapshot_commit"]):
        raise ValueError("Unpinned source revision")
    return obj


def inspect(manifest: dict, title: str | None) -> dict:
    games = [
        {
            "title_id": item["title_id"],
            "game": item["title"],
            "build_ids": [x["build_id"] for x in item["hamlet_builds"]],
            "named_sections": item["hamlet_named_sections_total"],
            "known_local_build": item["runtime_evidence"]["build_id"],
            "source": item["source_url"],
            "static_nsp_build_id": item.get("static_nsp_observation", {}).get("cheat_build_id"),
            "pinned_exact_build_sections": item.get("static_nsp_observation", {}).get("pinned_native_cheat_sections_matched", 0),
            "external_exact_build_sources": [
                {
                    "source_id": source["source_id"],
                    "build_id": source["build_id"],
                    "author_credit": source["author_credit"],
                    "source_url": source["source_url"],
                    "rights_status": source["rights_status"],
                    "body_imported": source["body_imported"],
                    "gameplay_verified": source["gameplay_verified"],
                }
                for source in item.get("exact_build_external_sources", [])
            ],
            "status": "source_candidates_only_not_runtime_verified",
        }
        for item in manifest["games"]
        if title is None or item["title_id"] == title
    ]
    if not games:
        raise ValueError("Game Title ID is not in the curated six-game inventory")
    return {"format": "cheatarium-switch-private-source-preview-v1", "writes": 0, "games": games}


def safe_output_root() -> Path:
    home = Path.home().resolve(strict=True)
    if not home.is_absolute():
        raise ValueError("No absolute home directory")
    return home / ".local/share/cheatarium/private-switch-curation"


def stage(manifest: dict, game: dict, root: Path) -> dict:
    """Private copy only. No Eden writes or installation; refuse overwrite."""
    title = game["title_id"]
    sha = manifest["upstream"]["snapshot_commit"]
    url = f"https://raw.githubusercontent.com/HamletDuFromage/switch-cheats-db/{sha}/cheats/{title}.json"
    with urlopen(Request(url, headers={"User-Agent": "Cheatarium-Private-Curation/1"}), timeout=30) as response:
        if response.geturl() != url:
            raise ValueError("Unexpected external redirect")
        raw = response.read(LIMIT + 1)
    if len(raw) > LIMIT:
        raise ValueError("Oversized upstream cheat JSON")
    obj = json.loads(raw)
    if not isinstance(obj, dict):
        raise ValueError("Unexpected upstream schema")
    expected = {build["build_id"]: build["named_sections"] for build in game["hamlet_builds"]}
    if {k for k in obj if BUILD_ID.fullmatch(k)} != set(expected):
        raise ValueError("Pinned Build ID list differs from the curated manifest")
    outputs = []
    pending = []
    for bid, sections in obj.items():
        if not BUILD_ID.fullmatch(bid):
            continue
        if not isinstance(sections, dict) or len(sections) != expected[bid]:
            raise ValueError("Pinned section count differs from curated manifest")
        accepted, rejected = parse_native(sections)
        if not accepted:
            raise ValueError(f"No native sections for {bid}")
        payload = "\n".join(text for _, text in accepted).encode("utf-8")
        if len(payload) > LIMIT:
            raise ValueError("Oversized staged cheat file")
        dest = root / title / f"{bid}.txt"
        if dest.is_symlink() or dest.parent.is_symlink():
            raise ValueError("Unexpected symlink in staging destination")
        if dest.exists():
            if not dest.is_file() or dest.read_bytes() != payload:
                raise ValueError(f"Existing private cache differs; refusing overwrite: {dest}")
            status = "already_staged_identical"
        else:
            status = "ready_to_stage"
            pending.append((dest, payload))
        outputs.append({
            "build_id": bid,
            "path": str(dest),
            "accepted_native_sections": len(accepted),
            "rejected_non_native_sections": rejected,
            "status": status,
        })
    # Validate all candidates before writing the first output. All target
    # filenames are source-declared Build IDs; no arbitrary path segments.
    for dest, payload in pending:
        dest.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        if dest.parent.is_symlink():
            raise ValueError("Private output folder is a symlink")
        fd = os.open(dest, os.O_CREAT | os.O_EXCL | os.O_WRONLY | getattr(os, "O_NOFOLLOW", 0), 0o600)
        try:
            with os.fdopen(fd, "wb") as stream:
                stream.write(payload)
                stream.flush()
                os.fsync(stream.fileno())
        except Exception:
            dest.unlink(missing_ok=True)
            raise
    return {"title_id": title, "source": url, "files": outputs}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--title-id", help="Curated Title ID (optional in preview, required to fetch)")
    parser.add_argument("--fetch-private", action="store_true", help="Download original codes into private non-Eden cache")
    args = parser.parse_args()
    manifest = load_manifest()
    if args.title_id and not re.fullmatch(r"01[0-9A-F]{14}", args.title_id):
        parser.error("--title-id must be an exact uppercase base Title ID")
    selected = next((g for g in manifest["games"] if g["title_id"] == args.title_id), None)
    if args.fetch_private:
        if selected is None:
            parser.error("--fetch-private requires a curated --title-id")
        result = {
            "format": "cheatarium-switch-private-staging-v1",
            "eden_config_modified": False,
            "eden_mods_modified": False,
            "runtime_compatibility_verified": False,
            "results": [stage(manifest, selected, safe_output_root())],
        }
    else:
        result = inspect(manifest, args.title_id)
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as exc:
        print(f"Cheatarium private curation: {exc}", file=sys.stderr)
        sys.exit(1)
