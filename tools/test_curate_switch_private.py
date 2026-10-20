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


    def test_bulk_exact_mode_selects_five_observed_builds_and_skips_smash(self):
        catalog = curator.load_manifest()
        calls = []

        def staged(_manifest, game, _root, exact_build=None):
            calls.append((game["title_id"], exact_build))
            return {"title_id": game["title_id"], "files": []}

        from contextlib import redirect_stdout
        output = io.StringIO()
        with (
            patch.object(curator, "load_manifest", return_value=catalog),
            patch.object(curator, "safe_output_root", return_value=Path("/unused/test")),
            patch.object(curator, "stage", side_effect=staged),
            patch.object(sys, "argv", ["curate_switch_private.py", "--fetch-personal-exact"]),
            redirect_stdout(output),
        ):
            self.assertEqual(curator.main(), 0)
        report = json.loads(output.getvalue())
        self.assertEqual(report["staged_game_count"], 5)
        self.assertFalse(report["eden_mods_modified"])
        self.assertFalse(report["runtime_compatibility_verified"])
        self.assertEqual(report["selection"], "static_nsp_build_only_not_runtime_version_verified")
        self.assertEqual([entry["title_id"] for entry in report["skipped"]], ["01006A800016E000"])
        expected = {
            (game["title_id"], game["static_nsp_observation"]["cheat_build_id"])
            for game in catalog["games"]
            if game["static_nsp_observation"]["pinned_native_cheat_sections_matched"] > 0
        }
        self.assertEqual(set(calls), expected)

    def test_exact_mode_writes_only_selected_build(self):
        sha = "a" * 40
        title = "0100000000010000"
        bid = "AAAAAAAAAAAAAAAA"
        other = "BBBBBBBBBBBBBBBB"
        game = {
            "title_id": title,
            "hamlet_builds": [
                {"build_id": bid, "named_sections": 1, "strict_native_hex_sections": 1},
                {"build_id": other, "named_sections": 1, "strict_native_hex_sections": 1},
            ],
            "static_nsp_observation": {
                "status": "main_nso_extracted_from_local_nsp",
                "cheat_build_id": bid,
                "pinned_native_cheat_sections_matched": 1,
            },
        }
        manifest = {"upstream": {"snapshot_commit": sha}}
        url = f"https://raw.githubusercontent.com/HamletDuFromage/switch-cheats-db/{sha}/cheats/{title}.json"
        body = {
            bid: {"[One]": "[One]\n04000000 00000001"},
            other: {"[Two]": "[Two]\n04000000 00000002"},
        }
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "private-switch-curation"
            with patch.object(curator, "urlopen", side_effect=lambda *_args, **_kwargs: FakeResponse(json.dumps(body).encode(), url)):
                report = curator.stage(manifest, game, root, exact_build=bid)
            self.assertEqual([file["build_id"] for file in report["files"]], [bid])
            self.assertTrue((root / title / (bid + ".txt")).is_file())
            self.assertFalse((root / title / (other + ".txt")).exists())

    def test_exact_mode_rejects_manifest_count_disagreement_without_writes(self):
        bid = "AAAAAAAAAAAAAAAA"
        game = {
            "title_id": "0100000000010000",
            "hamlet_builds": [{"build_id": bid, "named_sections": 1, "strict_native_hex_sections": 2}],
            "static_nsp_observation": {
                "status": "main_nso_extracted_from_local_nsp",
                "cheat_build_id": bid,
                "pinned_native_cheat_sections_matched": 1,
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "private-switch-curation"
            with self.assertRaisesRegex(ValueError, "section count disagrees"):
                curator.stage({"upstream": {"snapshot_commit": "a" * 40}}, game, target, exact_build=bid)
            self.assertFalse(target.exists())


if __name__ == "__main__":
    unittest.main()
