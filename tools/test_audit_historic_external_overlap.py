#!/usr/bin/env python3
"""Offline exact-byte historical-vs-external audit regression tests."""
import unittest

from audit_historic_external_overlap import compile_report
from audit_historic_libretro import HIST_REV


def corpus(rows):
    return {
        "format": "cheatarium-historical-libretro-unarchived-links-v1",
        "snapshot_commit": HIST_REV,
        "metadata_only": True, "code_payloads_included": False,
        "rights_status": "link_only_rights_review", "archived": False,
        "original_file_occurrences": len(rows),
        "distinct_git_blob_sha1": len({sha for _, sha, _ in rows}),
        "files": [
            {"source_path": "cht/DS/" + path, "git_blob_sha": sha,
             "bytes": size, "upstream_folder": "DS"}
            for path, sha, size in rows
        ],
    }


def external(source, files):
    return {"id": source, "platform": "ds",
            "rights_status": "link_only_rights_review", "files": files}


class HistoricalCrossCatalogTests(unittest.TestCase):
    def test_exact_blobs_not_names_and_union_not_sum(self):
        history = corpus([
            ("A.cht", "a" * 40, 4),
            ("B.cht", "a" * 40, 4),
            ("C.cht", "b" * 40, 8),
            ("D.cht", "c" * 40, 9),
        ])
        others = [
            external("other-one", [
                ["different-name.txt", "a" * 40, 4],
                ["different-name-copy.txt", "a" * 40, 4],
                ["archived.txt", "d" * 40, 5],
            ]),
            external("other-two", [
                ["same-name-different-byte.cht", "e" * 40, 6],
            ]),
        ]
        r = compile_report(history, others, {"d" * 40})
        self.assertEqual(r["historical_unarchived_file_occurrences"], 4)
        self.assertEqual(r["historical_distinct_blobs_with_external_byte_match"], 1)
        self.assertEqual(r["historical_file_occurrences_with_external_byte_match"], 2)
        self.assertEqual(r["historical_file_occurrences_without_external_byte_match"], 2)
        self.assertEqual(r["other_external_distinct_blobs_absent_from_archive"], 2)
        self.assertEqual(r["combined_unarchived_distinct_blobs"], 4)
        self.assertEqual(r["external_sources_with_matching_bytes"][0]["matching_historical_file_occurrences"], 2)
        self.assertEqual(r["external_sources_with_matching_bytes"][0]["matching_external_reference_occurrences"], 4)
        self.assertFalse(r["cheat_bodies_included"])

    def test_historical_rows_must_not_already_be_archived(self):
        with self.assertRaises(ValueError):
            compile_report(corpus([("A.cht", "a" * 40, 5)]), [], {"a" * 40})

    def test_matched_byte_sizes_must_agree(self):
        with self.assertRaises(ValueError):
            compile_report(corpus([("A.cht", "a" * 40, 5)]),
                           [external("other", [["match", "a" * 40, 6]])], set())

    def test_misleading_rights_or_stale_counts_are_rejected(self):
        h = corpus([("A.cht", "a" * 40, 5)])
        h["archived"] = True
        with self.assertRaises(ValueError):
            compile_report(h, [], set())
        h["archived"] = False
        h["original_file_occurrences"] = 2
        with self.assertRaises(ValueError):
            compile_report(h, [], set())

    def test_duplicated_external_path_rejected(self):
        with self.assertRaises(ValueError):
            compile_report(corpus([]), [
                external("other", [["same", "a" * 40, 5], ["same", "b" * 40, 5]])
            ], set())


if __name__ == "__main__":
    unittest.main()
