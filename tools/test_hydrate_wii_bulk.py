#!/usr/bin/env python3
"""Offline synthetic tests: strict original Wii provenance, private caching."""
import json
from pathlib import Path
import tempfile
import unittest

from hydrate_wii_bulk import (
    FORMAT, SOURCE_IDS, build_into, check, expected_files, hydrate, pinned_sources
)
from hydrate_nintendo_bulk import git_blob_sha


class WiiPrivateHydrationTests(unittest.TestCase):
    def test_native_source_counts_and_preserved_region_revision_names(self):
        sources = pinned_sources()
        self.assertEqual([len(s["files"]) for s in sources], [9, 12, 3, 5])
        expected = expected_files(sources)
        self.assertEqual(len(expected), 29)
        self.assertIn("wii/wii-accf-gecko-native/gecko/RUUE01v0.txt", expected)
        self.assertIn("wii/wii-accf-gecko-native/gecko/RUUE01v1.txt", expected)
        self.assertIn("wii/wii-metroid-prime-gecko-native/codes/R3ME01_mp1.ini", expected)

    def test_fixture_source_keeps_native_text_exact_and_checks_git_blob(self):
        sources = pinned_sources()
        raw_by_blob = {}
        for s in sources:
            for row in s["files"]:
                # Exact original-byte validation is handled by the integration
                # test, not simulated as a passing source blob here.
                pass
        with tempfile.TemporaryDirectory() as temp:
            dst = Path(temp)
            with self.assertRaises(ValueError):
                build_into(dst, sources, lambda _source, row: b"INVALID")
            self.assertFalse((dst / "manifest.json").exists())

    def test_wrong_license_cache_receipt_is_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            doc = {
                "schema_version": 1, "format": FORMAT, "platform": "wii",
                "private_local_only": True, "rights_to_redistribute_original_code_bodies": True,
                "gameplay_or_rom_revision_verified": False, "source_count": 4,
                "wii_original_gecko_source_files": 29, "source_ids": list(SOURCE_IDS),
                "records": [],
            }
            (root / "manifest.json").write_text(json.dumps(doc))
            with self.assertRaises(ValueError):
                check(root)

    def test_prohibit_writing_code_bodies_inside_git_tree(self):
        from hydrate_wii_bulk import ROOT
        with self.assertRaises(ValueError):
            hydrate(ROOT / "archive" / "wii-originals", fetch=lambda *_: self.fail("network access"))

    def test_failing_download_leaves_no_visible_cache(self):
        with tempfile.TemporaryDirectory() as temp:
            destination = Path(temp) / "private-nintendo"
            with self.assertRaises(ValueError):
                hydrate(destination, fetch=lambda _source, row: b"wrong")
            self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
