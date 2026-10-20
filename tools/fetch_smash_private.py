#!/usr/bin/env python3
"""Privately stage Smash Ultimate v13.0.2 native cheats from an attributed public post.

One exact Title ID + Build ID only. Never touches Eden or commits cheat bodies.
Only the user's local private cache receives the third-party source text.
"""
from __future__ import annotations

import argparse
import hashlib
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import re
import sys
from urllib.request import Request, build_opener, HTTPRedirectHandler

TITLE = "01006A800016E000"
BUILD = "CBD5A9B56EA859BA"
SOURCE = {
    "gbatemp-smash-1302-adaptation":
        "https://gbatemp.net/threads/super-smash-bros-ultimate-tid-01006a800016e000-bid-b9b166df1db90baf-v-13-0-1.608055/",
    "panda-smash-1302":
        "https://www.imwmi.com/cheats/2025.html",
}
HEADING_NAMES = [
    "1p invincible", "1p invincible off", "cpu no move", "cpu no move off",
    "hitkill on", "hitkill off", "run+speed normal", "run+speed 1.25x",
    "run+speed 1.5x", "run+speed 1.75x", "run+speed 2x",
    "jump up normal", "jump up 1.25x", "jump up 1.5x",
    "jump up 1.75x", "jump up 2x", "999.0 damager on",
    "999.0 damager off", "god mode on", "god mode off",
    "1 hit kill on", "1 hit kill off", "999999d money on",
    "999999d money off", "boss no move",
]
MAX_HTML = 2_000_000
MAX_CODE = 50_000
HEX_LINE = re.compile(r"^(?:[0-9a-fA-F]{8}[ \t]+){1,4}[0-9a-fA-F]{8}$")
HEADING = re.compile(r"^\[([^\[\]\r\n]{1,128})\]$")


class TextPage(HTMLParser):
    """Preserve the line layout of forum bbcode and preformatted cheat text."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []
        self.ignored = 0

    def handle_starttag(self, tag: str, attrs):
        if tag in {"script", "style", "noscript"}:
            self.ignored += 1
        elif not self.ignored and tag in {"br", "hr", "p", "pre", "blockquote", "div", "li"}:
            self.parts.append("\n")

    def handle_endtag(self, tag: str):
        if tag in {"script", "style", "noscript"}:
            self.ignored = max(0, self.ignored - 1)
        elif not self.ignored and tag in {"p", "pre", "blockquote", "div", "li"}:
            self.parts.append("\n")

    def handle_data(self, data: str):
        if not self.ignored:
            self.parts.append(data)


def extract(html_data: str) -> bytes:
    page = TextPage()
    page.feed(html_data)
    text = "".join(page.parts).replace("\r", "\n").replace("\xa0", " ")
    # A claim of this *specific* static NSP version must be visible in the
    # retrieved page, not inferred from a generic Smash title or old codes.
    if TITLE not in text.upper() or BUILD not in text.upper():
        raise ValueError("Page does not identify the expected Smash Title ID and Build ID")
    lines = [line.strip() for line in text.splitlines()]
    starts = [index for index, line in enumerate(lines)
              if HEADING.fullmatch(line) and line[1:-1].casefold() == HEADING_NAMES[0]]
    for start in starts:
        chunks = []
        cursor = start
        valid = True
        for index, expected in enumerate(HEADING_NAMES):
            if cursor >= len(lines):
                valid = False
                break
            heading = HEADING.fullmatch(lines[cursor])
            if heading is None or heading.group(1).casefold() != expected:
                valid = False
                break
            label = heading.group(1)
            cursor += 1
            codes = []
            while cursor < len(lines) and lines[cursor] == "":
                cursor += 1
            while cursor < len(lines) and HEX_LINE.fullmatch(lines[cursor]):
                codes.append(" ".join(lines[cursor].upper().split()))
                cursor += 1
                while cursor < len(lines) and lines[cursor] == "":
                    cursor += 1
            if not codes or len(codes) > 32:
                valid = False
                break
            chunks.append(f"[{label}]\n" + "\n".join(codes) + "\n")
            if index < len(HEADING_NAMES) - 1:
                if cursor >= len(lines) or not HEADING.fullmatch(lines[cursor]):
                    valid = False
                    break
        if valid:
            encoded = ("\n".join(chunks)).encode("utf-8")
            if len(encoded) > MAX_CODE:
                raise ValueError("Extracted cheat payload exceeds bound")
            if len(chunks) != 25:
                raise ValueError("Unexpected number of exact-build code sections")
            return encoded
    raise ValueError("Could not find the complete 25-section exact-build native code sequence")


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("Refusing third-party source redirect")


def fetch(source_id: str) -> bytes:
    url = SOURCE[source_id]
    opener = build_opener(NoRedirect)
    with opener.open(Request(url, headers={
        "User-Agent": "Cheatarium-Private-Switch-Curation/1",
        "Accept": "text/html",
    }), timeout=25) as response:
        if response.geturl() != url:
            raise ValueError("Unexpected source URL")
        data = response.read(MAX_HTML + 1)
    if len(data) > MAX_HTML:
        raise ValueError("Upstream HTML is too large")
    return data


def manifest_source() -> dict[str, str]:
    path = Path(__file__).resolve().parent.parent / "research/switch-personal-curation-v1.json"
    obj = json.loads(path.read_text(encoding="utf-8"))
    if obj.get("format") != "cheatarium-user-switch-cheat-source-curation-v1":
        raise ValueError("Unknown curation manifest")
    games = [g for g in obj["games"] if g["title_id"] == TITLE]
    if len(games) != 1:
        raise ValueError("Expected exactly one curated Smash game")
    game = games[0]
    evidence = game["static_nsp_observation"]
    if (evidence["status"] != "main_nso_extracted_from_local_nsp"
            or evidence["cheat_build_id"] != BUILD
            or evidence["pinned_native_cheat_sections_matched"] != 0):
        raise ValueError("Smash static NSP evidence or Hamlet source status changed")
    return {
        entry["source_id"]: entry["source_url"]
        for entry in game["exact_build_external_sources"]
        if entry["build_id"] == BUILD and entry["source_id"] in SOURCE
    }


def store(payload: bytes, source_id: str, url: str, root: Path) -> dict:
    if len(payload) > MAX_CODE or not payload:
        raise ValueError("Empty or oversized cheat file")
    title_dir = root / TITLE
    file = title_dir / (BUILD + ".txt")
    receipt = title_dir / (BUILD + ".source.json")
    digest = hashlib.sha256(payload).hexdigest()
    info = {
        "format": "cheatarium-private-external-native-source-v1",
        "title_id": TITLE,
        "build_id": BUILD,
        "source_id": source_id,
        "source_url": url,
        "section_count": len(HEADING_NAMES),
        "body_sha256": digest,
        "gameplay_verified": False,
    }
    meta = (json.dumps(info, indent=2) + "\n").encode("utf-8")
    for parent in (root, title_dir):
        if parent.is_symlink():
            raise ValueError("Private curation directory must not be a symlink")
    for path, content in ((file, payload), (receipt, meta)):
        if path.is_symlink():
            raise ValueError("Existing private curation file must not be a symlink")
        if path.exists() and (not path.is_file() or path.read_bytes() != content):
            raise ValueError(f"Existing private data differs; refusing overwrite: {path}")
    pending = [(path, content) for path, content in ((file, payload), (receipt, meta))
               if not path.exists()]
    title_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    if title_dir.is_symlink():
        raise ValueError("Private title directory became a symlink")
    created: list[Path] = []
    try:
        for path, content in pending:
            fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
            created.append(path)
            with os.fdopen(fd, "wb") as stream:
                stream.write(content)
                stream.flush()
                os.fsync(stream.fileno())
    except Exception:
        for path in created:
            path.unlink(missing_ok=True)
        raise
    return {
        "format": info["format"],
        "title_id": TITLE,
        "build_id": BUILD,
        "source_id": source_id,
        "source_url": url,
        "native_sections_staged": len(HEADING_NAMES),
        "body_sha256": digest,
        "status": "already_staged_identical" if not pending else "staged_privately",
        "eden_modified": False,
        "gameplay_verified": False,
    }


def main() -> int:
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--source", choices=tuple(SOURCE), help="Use only this attributed public source")
    args = cli.parse_args()
    known = manifest_source()
    selected = [args.source] if args.source else list(SOURCE)
    errors = []
    for source_id in selected:
        if known.get(source_id) != SOURCE[source_id]:
            raise ValueError(f"Pinned source URL mismatch for {source_id}")
        try:
            payload = extract(fetch(source_id).decode("utf-8", errors="replace"))
            root = Path.home().resolve(strict=True) / ".local/share/cheatarium/private-switch-curation"
            print(json.dumps(store(payload, source_id, SOURCE[source_id], root), indent=2))
            return 0
        except (OSError, UnicodeError, ValueError) as exc:
            errors.append(f"{source_id}: {exc}")
    raise ValueError("Failed to stage exact-build Smash cheats: " + "; ".join(errors))


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, KeyError, json.JSONDecodeError) as exc:
        print(f"Cheatarium Smash private acquisition: {exc}", file=sys.stderr)
        sys.exit(1)
