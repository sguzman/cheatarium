#!/usr/bin/env python3
"""Synthetic native 3DS file-label and uncertain-rights regression tests."""
import unittest
from audit_3ds_external_title_ids import compare, classify_link_source

A = "0004000000006200"
B = "0004000000007500"
C = "000400000014F100"
D = "000400000014F200"


def ext(sid, paths):
    return {"id": sid, "platform": "3ds", "snapshot_commit": "a"*40,
            "rights_status": "link_only_rights_review",
            "files": [[p, "a"*40, 100] for p in paths]}


class Audit3DSCoverage(unittest.TestCase):
    def setUp(self):
        self.original = {"id": "sharkive", "files": [
            {"upstream_path": f"3ds/{A}.txt"},
            {"upstream_path": f"3ds/{B}.txt"},
            {"upstream_path": "switch/0100000000000000/a.txt"},
        ]}
        self.external = [
            ext("ademola-3ds-native", [
                f"Game (USA)/{A}.txt", f"Game (EUR)/{C}.txt"
            ]),
            ext("journeyover-3ds-ctrpf", [
                f"Cheats/Game (USA)/{C}.txt", f"Cheats/Game (EUR)/{D}.txt"
            ]),
            ext("rogamer97-asm-3ds", [
                "3DS/Mario Kart 7/Infinite Items.md"
            ]),
            ext("riyoway-3ds-json", ["exports/JSON/Nintendo/3ds.json"]),
        ]

    def test_union_and_source_overlap(self):
        report = compare(self.original, self.external)
        totals = report["summary"]
        self.assertEqual(totals["external_native_title_id_file_occurrences"], 4)
        self.assertEqual(totals["external_distinct_title_id_labels"], 3)
        self.assertEqual(totals["title_ids_in_both_archive_and_external"], 1)
        self.assertEqual(totals["external_title_id_labels_absent_from_sharkive"], 2)
        self.assertEqual(totals["combined_distinct_3ds_title_id_labels"], 4)
        self.assertEqual(totals["non_native_3ds_code_documents"], 2)
        self.assertTrue(report["rights_blocked_external_codes_not_imported"])
        self.assertFalse(report["game_identity_verified"])
        self.assertEqual(report["external"][0]["source_titles_missing_from_archived_sharkive"], 1)

    def test_malformed_native_path_fails_closed(self):
        with self.assertRaises(ValueError):
            classify_link_source("ademola-3ds-native", [["MarioKart7.txt", "f"*40, 7]])
        with self.assertRaises(ValueError):
            compare(self.original, self.external + [ext("unexpected", ["foo.txt"])])

    def test_missing_reviewed_source_fails_closed(self):
        with self.assertRaises(ValueError):
            compare(self.original, self.external[:2])


if __name__ == "__main__":
    unittest.main()
