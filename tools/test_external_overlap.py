#!/usr/bin/env python3
"""Invariants for the exact-blob external reference overlap report."""
import unittest

from audit_external_overlap import build


class TestExternalOverlap(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.data = build()

    def test_fixed_pinned_inventory(self):
        doc = self.data
        self.assertEqual(doc["reference_files"], 43409)
        self.assertEqual(doc["source_catalogs"], 48)
        self.assertEqual(doc["external_platforms"], 26)
        self.assertGreater(doc["distinct_git_blobs"], 26638)
        self.assertEqual(doc["repeated_blob_excess_occurrences"], 43409 - doc["distinct_git_blobs"])
        self.assertGreaterEqual(doc["references_in_repeated_blob_groups"], 9076)
        self.assertGreaterEqual(doc["cross_source_shared_blob_groups"], 2759)
        self.assertGreaterEqual(doc["cross_source_shared_references"], 8129)
        self.assertTrue(doc["metadata_only"])
        self.assertFalse(doc["archive_payloads_included"])
        self.assertFalse(doc["independently_verified_games_or_effects"])

    def test_platform_and_source_counts_reconcile(self):
        doc = self.data
        self.assertEqual(sum(x["reference_files"] for x in doc["sources"]), 43409)
        self.assertEqual(sum(x["reference_files"] for x in doc["platforms"]), 43409)
        hamlet = next(x for x in doc["sources"] if x["id"] == "hamlet-switch-cheatdb")
        self.assertEqual((hamlet["reference_files"], hamlet["distinct_git_blobs"]), (5278, 3046))
        ds = next(x for x in doc["sources"] if x["id"] == "deadskullz-nds-formats")
        self.assertEqual(ds["reference_files"], 9)
        self.assertEqual(ds["distinct_git_blobs"], 8)
        self.assertEqual(ds["repeated_blob_excess_occurrences_within_source"], 1)
        fba = next(x for x in doc["sources"] if x["id"] == "smoker1-fba-official-ini")
        self.assertEqual((fba["reference_files"], fba["distinct_git_blobs"]), (98, 45))
        self.assertEqual(fba["repeated_blob_excess_occurrences_within_source"], 53)
        smoker = [x for x in doc["sources"] if x["id"].startswith("smoker1-legacy-")]
        self.assertEqual(sum(x["reference_files"] for x in smoker), 75)
        self.assertEqual(sum(x["repeated_blob_excess_occurrences_within_source"]
                             for x in smoker), 5)
        gcm = [x for x in doc["sources"] if x["id"].startswith("gcmcom-legacy-")]
        self.assertEqual(sum(x["reference_files"] for x in gcm), 11)
        sthetix = next(x for x in doc["sources"] if x["id"] == "sthetix-switch-mirror")
        self.assertEqual((sthetix["reference_files"], sthetix["distinct_git_blobs"]), (5861, 5468))
        self.assertEqual(
            sum(x["repeated_blob_excess_occurrences_within_source"] for x in doc["sources"]), 3384
        )

    def test_cross_source_matches_keep_paths_and_no_bodies(self):
        doc = self.data
        matched = doc["cross_source_exact_blob_matches"]
        self.assertEqual(len(matched), doc["cross_source_shared_blob_groups"])
        previous_matches = {
            "bd9c87c6fcebcd47abd376d76cc6628d03c44019",
            "c4b7d0f01961d2ca240e8f3b6a6e0df10274d5fc",
        }
        self.assertTrue(previous_matches.issubset({x["git_blob_sha"] for x in matched}))
        self.assertTrue(any(
            {"sthetix-switch-mirror", "hamlet-switch-cheatdb"}.issubset(
                {r["source_id"] for r in x["references"]}
            ) for x in matched
        ))
        for row in matched:
            self.assertGreaterEqual(row["source_count"], 2)
            self.assertGreaterEqual(len(row["references"]), 2)
            for ref in row["references"]:
                self.assertEqual(
                    set(ref), {"source_id", "platform", "path", "git_blob_sha", "bytes"}
                )

    def test_reproducible(self):
        self.assertEqual(self.data, build())


if __name__ == "__main__":
    unittest.main()
