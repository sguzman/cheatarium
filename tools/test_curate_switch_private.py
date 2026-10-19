#!/usr/bin/env python3
"""Offline tests for the six-title private, pinned Switch source curator."""
from __future__ import annotations

import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import curate_switch_private as curator


class FakeResponse(io.BytesIO):
    def __init__(self, content: bytes, url: str):
        super().__init__(content)
        self.url = url

    def geturl(self) -> str:
        return self.url


class PrivateCuratorTests(unittest.TestCase):
    def test_source_registry_six_titles_and_no_runtime_invention(self):
        catalog = curator.load_manifest()
        summary = curator.inspect(catalog, None)
        self.assertEqual(len(summary["games"]), 6)
        self.assertEqual(summary["writes"], 0)
        titles = {x["title_id"] for x in summary["games"]}
        self.assertIn("01002DA013484000", titles)
        self.assertIn("0100BC0018138000", titles)
        self.assertEqual(
            {x["title_id"] for x in catalog["games"] if x["runtime_evidence"]["build_id"]},
            {"0100000000010000"},
        )
        skyward = curator.inspect(catalog, "01002DA013484000")
        self.assertEqual(
            set(skyward["games"][0]["build_ids"]),
            {"5441AF3C89F97E1D", "D824D4B1BBD45D03"},
        )
        with self.assertRaises(ValueError):
            curator.inspect(catalog, "0100DEADBEEF0000")

    def test_only_native_hex_sections_are_staged(self):
        accepted, rejected = curator.parse_native({
            "[Unlimited HP]": "[Unlimited HP]\n04000000 06244B58 000F4240\n",
            "[Instructions]": "[Instructions]\nNot executable\n",
            "[Bad]": "[Another label]\n04000000 00000000 00000000",
            "[Controller]": "[Controller]\n80000040\nFF000000\n20000000\n",
        })
        self.assertEqual(len(accepted), 2)
        self.assertEqual(len(rejected), 2)
        self.assertEqual(accepted[0][0], "[Unlimited HP]")
        self.assertEqual(accepted[1][0], "[Controller]")

    def test_private_staging_pins_build_ids_and_refuses_overwrite(self):
        sha = "a" * 40
        title = "01002DA013484000"
        bid = "5441AF3C89F97E1D"
        game = {"title_id": title, "hamlet_builds": [{"build_id": bid, "named_sections": 2}]}
        manifest = {"upstream": {"snapshot_commit": sha}}
        url = f"https://raw.githubusercontent.com/HamletDuFromage/switch-cheats-db/{sha}/cheats/{title}.json"
        data = {
            bid: {
                "[Infinite Stamina]": "[Infinite Stamina]\n04000000 06244B58 000F4240\n",
                "[Description]": "[Description]\nA human explanation",
            },
            "attribution": {"source": "source authors"},
        }
        raw = json.dumps(data).encode("utf-8")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            with patch.object(curator, "urlopen", side_effect=lambda *_args, **_kwargs: FakeResponse(raw, url)):
                result = curator.stage(manifest, game, path)
                self.assertEqual(result["files"][0]["accepted_native_sections"], 1)
                self.assertEqual(result["files"][0]["rejected_non_native_sections"], ["[Description]"])
                dest = path / title / (bid + ".txt")
                self.assertTrue(dest.is_file())
                self.assertNotIn("Description", dest.read_text())
                repeat = curator.stage(manifest, game, path)
                self.assertEqual(repeat["files"][0]["status"], "already_staged_identical")
                dest.write_text("USER MODIFIED", encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "refusing overwrite"):
                    curator.stage(manifest, game, path)
                self.assertEqual(dest.read_text(), "USER MODIFIED")


if __name__ == "__main__":
    unittest.main()
