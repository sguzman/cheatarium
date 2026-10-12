#!/usr/bin/env python3
"""Offline regression tests for historical Libretro fork comparison."""
import unittest

from audit_historic_libretro import (
    HIST_TREE, parse_cheat_tree, compile_report, validate_report,
)


def blob(name, sha, size=1, mode="100644"):
    return {"path": name, "sha": sha * 40, "size": size,
            "type": "blob", "mode": mode}


class HistoricalForkTests(unittest.TestCase):
    def test_archive_delta_and_path_comparisons_are_distinct(self):
        h = [
            ("NES/A.cht", "a" * 40, 1),
            ("NES/B.cht", "b" * 40, 2),
            ("DS/C.cht", "c" * 40, 3),
            ("DS/D.cht", "d" * 40, 4),
            ("DS/E.cht", "d" * 40, 4),
        ]
        modern = [
            ("NES/A.cht", "a" * 40, 1),
            ("NES/B.cht", "e" * 40, 2),
            ("NES/other.cht", "f" * 40, 3),
        ]
        r = compile_report(h, modern, {"b" * 40})
        self.assertEqual(r["same_path_same_blob_occurrences"], 1)
        self.assertEqual(r["same_path_different_blob_occurrences"], 1)
        self.assertEqual(r["historical_paths_absent_from_baseline"], 3)
        self.assertEqual(r["historical_occurrences_with_blob_absent_from_baseline"], 4)
        self.assertEqual(r["baseline_absent_occurrences_already_in_bithex"], 1)
        self.assertEqual(r["candidate_occurrences_absent_from_both"], 3)
        self.assertEqual(r["candidate_distinct_blobs_absent_from_both"], 2)
        self.assertEqual(r["candidate_occurrences_by_historical_folder"], {"DS": 3})
        self.assertFalse(r["historical_code_redistribution_rights_confirmed"])

    def test_truncated_and_wrong_tree_rejected(self):
        good = {"sha": HIST_TREE, "truncated": False, "tree": [blob("DS/A.cht", "a")]}
        self.assertEqual(len(parse_cheat_tree(good, HIST_TREE)), 1)
        with self.assertRaises(ValueError):
            parse_cheat_tree({**good, "truncated": True}, HIST_TREE)
        with self.assertRaises(ValueError):
            parse_cheat_tree({**good, "sha": "0" * 40}, HIST_TREE)

    def test_path_traversal_duplicate_and_symlink_rejected(self):
        for bad in [
            [blob("../evil.cht", "b")],
            [blob("DS/A.cht", "b"), blob("DS/A.cht", "c")],
            [blob("DS/A.cht", "b", mode="120000")],
            [blob("DS/A.cht", "not-a-sha")],
        ]:
            with self.assertRaises(ValueError):
                parse_cheat_tree({"sha": HIST_TREE, "truncated": False, "tree": bad}, HIST_TREE)

    def test_non_cheat_metadata_skipped(self):
        payload = {"sha": HIST_TREE, "truncated": False,
                   "tree": [blob("README.md", "a"),
                            {"path": "NES", "type": "tree", "sha": "f" * 40},
                            blob("NES/One.cht", "b")]}
        self.assertEqual(len(parse_cheat_tree(payload, HIST_TREE)), 1)

    def test_integrity_and_rights_assertions(self):
        r = compile_report([("NES/A.cht", "a" * 40, 1)], [], set())
        r["candidate_occurrences_by_historical_folder"] = {"NES": 2}
        with self.assertRaises(ValueError):
            validate_report(r)
        r = compile_report([("NES/A.cht", "a" * 40, 1)], [], set())
        r["historical_code_redistribution_rights_confirmed"] = True
        with self.assertRaises(ValueError):
            validate_report(r)


if __name__ == "__main__":
    unittest.main()
