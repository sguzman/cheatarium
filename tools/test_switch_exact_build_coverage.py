#!/usr/bin/env python3
"""Offline invariants for the pinned six-title Switch exact-Build-ID coverage report."""
from __future__ import annotations

import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "research/switch-exact-build-coverage-v1.json"
MANIFEST = ROOT / "research/switch-personal-curation-v1.json"
EXPECTED = {
    "0100000000010000": 9,
    "0100152000022000": 2,
    "01002DA013484000": 47,
    "01006A800016E000": 25,
    "01006BB00C6F0000": 38,
    "0100BC0018138000": 53,
}


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


class SwitchExactBuildCoverageTests(unittest.TestCase):
    def test_exact_build_source_record_and_archival_coverage(self):
        report = read_json(REPORT)
        manifest = read_json(MANIFEST)
        self.assertEqual(report["schema_version"], 1)
        self.assertEqual(report["format"], "cheatarium-switch-exact-build-coverage-v1")
        self.assertTrue(report["metadata_only_for_unarchived_sections"])
        self.assertFalse(report["gameplay_verified"])
        by_title = {game["title_id"]: game for game in manifest["games"]}
        self.assertEqual(set(by_title), set(EXPECTED))
        self.assertEqual({game["title_id"] for game in report["games"]}, set(EXPECTED))
        self.assertEqual(len(report["games"]), len(EXPECTED))

        archived, missing = 0, 0
        for game in report["games"]:
            tid, bid = game["title_id"], game["build_id"]
            with self.subTest(title_id=tid):
                self.assertEqual(game["title"], by_title[tid]["title"])
                self.assertEqual(bid, by_title[tid]["static_nsp_observation"]["cheat_build_id"])
                self.assertEqual(game["section_count"], EXPECTED[tid])
                self.assertEqual(len(game["sections"]), EXPECTED[tid])
                self.assertTrue(game["curated_file"].startswith("curated/switch/"))
                self.assertNotIn("..", Path(game["curated_file"]).parts)
                curated_path = ROOT / game["curated_file"]
                if curated_path.is_file():
                    curated = read_json(curated_path)
                    variants = {
                        cheat["id"]: variant
                        for cheat in curated["cheats"]
                        for variant in cheat["variants"]
                        if variant["edition"] == "build-" + bid.lower()
                    }
                else:
                    variants = {}
                seen = set()
                game_archived, game_missing = 0, 0
                for row in game["sections"]:
                    self.assertTrue(row["heading"].startswith("["))
                    self.assertTrue(row["heading"].endswith("]"))
                    self.assertNotIn(row["heading"], seen)
                    seen.add(row["heading"])
                    self.assertGreater(row["ordinal"], 0)
                    self.assertLessEqual(row["ordinal"], len(game["sections"]))
                    if row["state"] == "archived_code_exact_match":
                        game_archived += 1
                        key = row["curated_cheat_id"]
                        self.assertIn(key, variants)
                        archive_path = row["archive_path"]
                        self.assertEqual(variants[key]["source"]["archive_path"], archive_path)
                        self.assertTrue(archive_path.startswith("archive/"))
                        self.assertNotIn("..", Path(archive_path).parts)
                        self.assertTrue((ROOT / archive_path).is_file())
                        self.assertEqual(variants[key]["status"], "unverified")
                    else:
                        self.assertEqual(row["state"], "linked_only_no_public_code_body")
                        game_missing += 1
                        self.assertIsNone(row["curated_cheat_id"])
                        self.assertIsNone(row["archive_path"])
                self.assertEqual(game_archived, game["archived_exact_section_count"])
                self.assertEqual(game_missing, game["missing_body_count"])
                self.assertEqual(game_archived + game_missing, EXPECTED[tid])
                if tid == "01006A800016E000":
                    self.assertEqual(game["source"]["kind"], "independent_exact_build_external")
                    self.assertGreaterEqual(game_archived, 0)
                    self.assertTrue(all(x["url"].startswith("https://") for x in game["source"]["sources"]))
                else:
                    self.assertEqual(game["source"]["kind"], "pinned_hamlet_mirror")
                    self.assertEqual(game["source"]["revision"], manifest["upstream"]["snapshot_commit"])
                    self.assertTrue(game["source"]["url"].endswith("/cheats/" + tid + ".json"))
                archived += game_archived
                missing += game_missing

        self.assertEqual(report["counts"], {
            "games": len(EXPECTED),
            "source_sections": sum(EXPECTED.values()),
            "archived_exact_sections": archived,
            "linked_only_sections": missing,
        })
        self.assertEqual(archived + missing, 174)
        self.assertGreater(archived, 0)
        self.assertGreaterEqual(missing, 0)


if __name__ == "__main__":
    unittest.main()
