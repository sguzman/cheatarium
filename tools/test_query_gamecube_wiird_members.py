#!/usr/bin/env python3
"""Read-only region/title lookup, no source downloads or executable cheats."""
import copy
import json
import unittest

from query_gamecube_wiird_members import query, DEFAULT


class GameCubeMemberQueryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(DEFAULT.read_text("utf-8"))

    def test_native_regions_and_pagination(self):
        for region, count in (("NTSC-J", 8), ("NTSC-U", 138), ("PAL", 229)):
            with self.subTest(region=region):
                result = query(self.doc, ".txt", region, 0, 2)
                self.assertEqual(result["total"], count)
                self.assertEqual(result["returned"], 2)
                self.assertTrue(result["has_more"])
                self.assertTrue(all(r["region"] == region for r in result["records"]))
                self.assertFalse(result["code_bodies_included"])
                self.assertFalse(result["game_identity_or_cheat_effects_verified"])

    def test_title_hint_search_and_out_of_range(self):
        candidate = self.doc["records"][0]["source_filename_title_hint"]
        result = query(self.doc, candidate, None, 0, 20)
        self.assertGreaterEqual(result["total"], 1)
        self.assertEqual(query(self.doc, ".txt", None, 375, 10)["returned"], 0)

    def test_rejects_tampered_scope_and_invalid_pagination(self):
        invalid = copy.deepcopy(self.doc)
        invalid["cheat_bodies_included"] = True
        with self.assertRaises(ValueError):
            query(invalid, "")
        with self.assertRaises(ValueError):
            query(self.doc, "", "USA")
        with self.assertRaises(ValueError):
            query(self.doc, "", None, -1)


if __name__ == "__main__":
    unittest.main()
