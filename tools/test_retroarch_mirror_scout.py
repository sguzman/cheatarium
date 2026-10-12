#!/usr/bin/env python3
"""Regression tests for strictly metadata-only Libretro-fork mirror leads."""
import copy
import json
import unittest

from audit_retroarch_mirror_scout import FILE, validate_offline


class TestLegacyMirrorScouting(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads(FILE.read_text("utf-8"))

    def test_pinned_source_counts_and_no_code_bodies(self):
        doc = self.doc
        validate_offline(doc)
        self.assertEqual(doc["file_occurrences_not_matching_three_corpora"], 86)
        self.assertEqual(doc["distinct_blob_sha1_in_union_not_matching_three_corpora"], 77)
        self.assertEqual(len(doc["sources"]), 2)
        gcm, smoker = doc["sources"]
        self.assertEqual((gcm["original_cht_file_occurrences"],
                          gcm["occurrences_matching_historical_exact_bytes"],
                          gcm["file_occurrences_not_matching_three_corpora"]), (10872, 10861, 11))
        self.assertEqual((smoker["original_cht_file_occurrences"],
                          smoker["file_occurrences_not_matching_three_corpora"],
                          smoker["distinct_blob_sha1_not_matching_three_corpora"]), (518, 75, 66))
        self.assertFalse(doc["cheat_or_archive_bodies_included"])
        self.assertFalse(doc["absence_from_all_other_archived_corpora_verified"])
        self.assertFalse(doc["gameplay_compatibility_verified"])

    def test_rejects_forged_rights_or_payloads(self):
        for key in ["rights_clearance_asserted", "cheat_or_archive_bodies_included",
                    "absence_from_all_other_archived_corpora_verified"]:
            bad = copy.deepcopy(self.doc)
            bad[key] = True
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate_offline(bad)
        bad = copy.deepcopy(self.doc)
        bad["sources"][0]["redistribution_cleared"] = True
        with self.assertRaises(ValueError):
            validate_offline(bad)

    def test_rejects_broken_provenance_and_counts(self):
        for property_, value in [
            ("git_blob_sha", "not-a-sha"),
            ("bytes", -1),
            ("original_folder", "invented"),
            ("pinned_upstream_url", "https://example.com/unpinned"),
            ("source_path", "../escaping.cht"),
        ]:
            bad = copy.deepcopy(self.doc)
            bad["sources"][0]["unmatched_source_file_metadata"][0][property_] = value
            with self.subTest(property_=property_), self.assertRaises(ValueError):
                validate_offline(bad)
        bad = copy.deepcopy(self.doc)
        bad["sources"][1]["unmatched_source_file_metadata"].pop()
        with self.assertRaises(ValueError):
            validate_offline(bad)


if __name__ == "__main__":
    unittest.main()
