#!/usr/bin/env python3
"""Safety and reproducibility checks for the pinned bulk scout."""
import copy
import json
import unittest
from audit_bulk_source_scout import FILE, verify, selected

class TestBulkSourceScout(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(FILE.read_text("utf-8"))

    def test_counts_and_rights(self):
        self.assertTrue(verify(self.doc))
        self.assertEqual(self.doc["source_file_occurrences"], 691)
        self.assertEqual(self.doc["distinct_git_blobs_in_source_union"], 487)
        self.assertEqual([s["selected_file_occurrences"] for s in self.doc["sources"]],
                         [621, 7, 51, 12])
        self.assertEqual(self.doc["sources"][0]["matching_smoker1_file_occurrences"], 0)

    def test_rejects_claimed_rights_or_function(self):
        for key in ("cheat_bodies_included", "licensed_to_archive",
                    "game_identity_verified", "cheat_function_verified",
                    "archive_novelty_verified"):
            bad = copy.deepcopy(self.doc)
            bad[key] = True
            with self.subTest(key=key), self.assertRaises(ValueError):
                verify(bad)
        bad = copy.deepcopy(self.doc)
        bad["sources"][1]["reuse_cleared"] = True
        with self.assertRaises(ValueError):
            verify(bad)

    def test_rejects_corruption_and_executable_paths(self):
        for value in ("../escape.dat", "cheats/trainer.exe", "cheats/fake.dat"):
            self.assertFalse(selected("eignar17-arcade", value))
        for index, invalid in ((0, "../escape.dat"), (1, "wrong-sha"), (2, -1)):
            bad = copy.deepcopy(self.doc)
            bad["sources"][0]["files"][0][index] = invalid
            with self.subTest(index=index), self.assertRaises(ValueError):
                verify(bad)
        bad = copy.deepcopy(self.doc)
        bad["sources"][0]["files"].append(bad["sources"][0]["files"][-1])
        with self.assertRaises(ValueError):
            verify(bad)

if __name__ == "__main__":
    unittest.main()
