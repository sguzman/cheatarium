#!/usr/bin/env python3
"""Check that curated Switch native codes exactly match archived source sections."""
from __future__ import annotations

import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
NATIVE = re.compile(r"^[0-9a-fA-F]{8}(?:\s+[0-9a-fA-F]{8}){0,4}$")
HEADING = re.compile(r"^\[([^\]\r\n]+)\]$")


def archived_sections(path: Path) -> set[tuple[str, tuple[str, ...]]]:
    out: set[tuple[str, tuple[str, ...]]] = set()
    heading: str | None = None
    codes: list[str] = []
    def flush() -> None:
        if heading and codes:
            out.add((heading, tuple(codes)))
    for original in path.read_text(encoding="utf-8").splitlines():
        line = original.strip()
        match = HEADING.fullmatch(line)
        if match:
            flush()
            heading = re.sub(r"^♯\s*", "", match.group(1).strip()).strip()
            codes = []
        elif heading and NATIVE.fullmatch(line):
            codes.append(line)
    flush()
    return out


class SwitchCurationTests(unittest.TestCase):
    def test_preserved_exact_native_sections(self):
        paths = sorted((ROOT / "curated/switch").glob("*/cheats.json"))
        self.assertGreaterEqual(len(paths), 5)
        total = 0
        for path in paths:
            with self.subTest(file=str(path.relative_to(ROOT))):
                obj = json.loads(path.read_text(encoding="utf-8"))
                self.assertEqual(obj["platform"], "switch")
                self.assertEqual(obj["game"]["id"], path.parent.name)
                editions = {x["id"]: x for x in obj["editions"]}
                found = set()
                archived: dict[str, set] = {}
                for cheat in obj["cheats"]:
                    self.assertNotIn(cheat["id"], found)
                    found.add(cheat["id"])
                    for variant in cheat["variants"]:
                        self.assertEqual(variant["status"], "unverified")
                        self.assertEqual(variant["format"], "atmosphere-native-switch")
                        edition = editions[variant["edition"]]
                        self.assertEqual(variant["edition"], "build-" + edition["build_id"].lower())
                        source = variant["source"]
                        rel = source["archive_path"]
                        self.assertTrue(rel.startswith("archive/"))
                        self.assertIn("/" + edition["build_id"].lower() + ".txt", rel.lower())
                        full = ROOT / rel
                        self.assertTrue(full.is_file())
                        if rel not in archived:
                            archived[rel] = archived_sections(full)
                        section = (cheat["name"], tuple(variant["code"].splitlines()))
                        self.assertIn(section, archived[rel], f"{cheat['name']} not in {rel}")
                        total += 1
        self.assertGreaterEqual(total, 200)


if __name__ == "__main__":
    unittest.main()
