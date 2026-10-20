#!/usr/bin/env python3
"""Synthetic offline tests for private Switch collection persistence."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import private_switch_collection as collection

TITLE = "0100000000010000"
BUILD = "B424BE150A8E7D78"
SMASH = "01006A800016E000"
SMASH_BUILD = "CBD5A9B56EA859BA"
SMASH_SOURCE = "https://example.com/smash"


def manifest() -> dict:
    return {
        "format": "cheatarium-user-switch-cheat-source-curation-v1",
        "upstream": {"snapshot_commit": "a" * 40},
        "games": [
            {
                "title_id": TITLE,
                "title": "Example platform game",
                "source_url": "https://example.com/hamlet",
                "hamlet_builds": [{"build_id": BUILD, "strict_native_hex_sections": 1}],
            },
            {
                "title_id": SMASH,
                "title": "Example fighting game",
                "hamlet_builds": [],
                "exact_build_external_sources": [
                    {"source_id": "example-external", "source_url": SMASH_SOURCE, "build_id": SMASH_BUILD}
                ],
            },
        ],
    }


class CollectionTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.base = Path(self.tmp.name)
        self.root = self.base / "collection"
        self.root.mkdir()
        self.manifest = self.base / "manifest.json"
        self.manifest.write_text(json.dumps(manifest()), encoding="utf-8")
        self.config = self.base / "config.json"
        folder = self.root / TITLE
        folder.mkdir()
        (folder / (BUILD + ".txt")).write_text(
            "[Infinite Energy]\n04000000 00000000 00000001\n", encoding="utf-8"
        )

    def test_index_search_and_repeat(self):
        index, files = collection.collect(self.root, self.manifest)
        self.assertEqual(index["game_count"], 1)
        self.assertEqual(index["section_count"], 1)
        self.assertEqual(index["records"][0]["sections"][0]["name"], "Infinite Energy")
        self.assertEqual(len(files), 1)
        first = collection.refresh_collection(self.root, self.manifest, self.config)
        self.assertEqual(first["backup"]["status"], "not_configured")
        second = collection.refresh_collection(self.root, self.manifest, self.config)
        self.assertEqual(first, second)
        self.assertEqual(
            json.loads((self.root / "collection-index.json").read_text())["section_count"], 1
        )

    def test_verified_append_only_backup_and_mismatch(self):
        index, files = collection.collect(self.root, self.manifest)
        payload = collection.write_index(self.root, index)
        backup = self.base / "independent-backup"
        result = collection.back_up(self.root, files, payload, backup)
        self.assertEqual(result["file_count"], 1)
        collection.back_up(self.root, files, payload, backup, verify_only=True)
        copied = backup / TITLE / (BUILD + ".txt")
        copied.write_text("corrupted")
        with self.assertRaisesRegex(ValueError, "conflict"):
            collection.back_up(self.root, files, payload, backup)
        self.assertEqual(copied.read_text(), "corrupted")

    def test_receipt_integrity(self):
        payload = b"[Invincible]\n04000000 00000000 00000001\n"
        folder = self.root / SMASH
        folder.mkdir()
        (folder / (SMASH_BUILD + ".txt")).write_bytes(payload)
        receipt = folder / (SMASH_BUILD + ".source.json")
        data = {
            "format": "cheatarium-private-external-native-source-v1",
            "title_id": SMASH,
            "build_id": SMASH_BUILD,
            "source_id": "example-external",
            "source_url": SMASH_SOURCE,
            "body_sha256": hashlib.sha256(payload).hexdigest(),
            "section_count": 1,
            "gameplay_verified": False,
        }
        receipt.write_text(json.dumps(data), encoding="utf-8")
        index, files = collection.collect(self.root, self.manifest)
        self.assertEqual(index["section_count"], 2)
        self.assertIn(receipt.relative_to(self.root), files)
        data["body_sha256"] = "0" * 64
        receipt.write_text(json.dumps(data), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "Invalid external source receipt"):
            collection.collect(self.root, self.manifest)

    def test_no_symlink_or_arbitrary_path_traversal(self):
        sibling = self.base / "elsewhere"
        sibling.mkdir()
        (self.root / "0100000000000001").symlink_to(sibling, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "Unexpected private collection entry"):
            collection.collect(self.root, self.manifest)

    def test_backup_configuration_round_trip(self):
        backup = self.base / "independent"
        collection.remember_backup(backup, self.config)
        self.assertEqual(collection.saved_backup(self.config), backup)
        result = collection.refresh_collection(self.root, self.manifest, self.config)
        self.assertEqual(result["backup"]["status"], "backed_up")
        self.assertTrue((backup / TITLE / (BUILD + ".txt")).is_file())


if __name__ == "__main__":
    unittest.main()
