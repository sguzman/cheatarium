#!/usr/bin/env python3
"""Tests for archive-wide pinned scout exact-byte gap accounting."""
import copy
import unittest

from build_bulk_scout_overlap import compile_report


def source(sid, rows):
    return {
        "id": sid, "rights_status": "link_only_rights_review",
        "reuse_cleared": False, "source_code_included": False,
        "files": rows, "selected_file_occurrences": len(rows),
        "distinct_git_blobs": len({x[1] for x in rows}),
    }


class ScoutOverlapTests(unittest.TestCase):
    def setUp(self):
        self.a, self.b, self.c, self.d = (
            "a" * 40, "b" * 40, "c" * 40, "d" * 40)
        self.archived = [(self.a, "preserved", "archive/preserved/a"),
                         (self.d, "preserved", "archive/preserved/d")]
        self.external = [
            {"id": "external", "files": [
                ["old.txt", self.b, 10], ["both.txt", self.d, 10]]}
        ]
        self.scouts = [
            source("alpha", [["archive.ini", self.a, 10],
                             ["external.ini", self.b, 10],
                             ["new.ini", self.c, 10],
                             ["alias.ini", self.c, 10]]),
            source("beta", [["both.zip", self.d, 10]]),
        ]

    def test_exact_occurrences_and_union_without_double_counting(self):
        report = compile_report(self.scouts, self.archived, self.external)
        self.assertEqual(report["scout_source_file_occurrences"], 5)
        self.assertEqual(report["scout_distinct_git_blobs"], 4)
        self.assertEqual(report["scout_distinct_git_blobs_matching_archive"], 2)
        self.assertEqual(report["scout_distinct_git_blobs_matching_existing_external"], 2)
        self.assertEqual(report["scout_distinct_git_blobs_not_in_either_known_corpus"], 1)
        alpha = report["sources"][0]
        self.assertEqual(alpha["repeated_byte_occurrences_within_source"], 1)
        self.assertEqual(alpha["occurrences_matching_archived_original_bytes"], 1)
        self.assertEqual(alpha["occurrences_matching_existing_external_bytes"], 1)
        self.assertEqual(alpha["occurrences_matching_either_known_corpus"], 2)
        self.assertEqual(alpha["distinct_blobs_not_in_either_known_corpus"], 1)
        beta = report["sources"][1]
        self.assertEqual(beta["occurrences_matching_either_known_corpus"], 1)
        self.assertFalse(report["rights_to_redistribute_established"])
        self.assertFalse(report["source_payloads_included"])
        self.assertFalse(report["gameplay_compatibility_verified"])

    def test_rejects_invalid_scoped_metadata(self):
        tamper = copy.deepcopy(self.scouts)
        tamper[0]["reuse_cleared"] = True
        with self.assertRaises(ValueError):
            compile_report(tamper, self.archived, self.external)
        for field, value in ((0, "../bad.ini"), (1, "invalid"), (2, -1)):
            tamper = copy.deepcopy(self.scouts)
            tamper[0]["files"][0][field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                compile_report(tamper, self.archived, self.external)
        with self.assertRaises(ValueError):
            compile_report(self.scouts + [self.scouts[0]], self.archived, self.external)
        with self.assertRaises(ValueError):
            compile_report(self.scouts, [("bad", "x", "y")], self.external)

    def test_no_invented_new_games_from_different_bytes(self):
        result = compile_report(self.scouts, self.archived, self.external)
        self.assertFalse(result["new_games_or_cheat_effects_verified"])
        self.assertEqual(result["coverage_universe"],
                         "pinned preserved originals and preexisting external catalogs only")


if __name__ == "__main__":
    unittest.main()
