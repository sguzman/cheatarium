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
        self.assertEqual(doc["reference_files"], 22886)
        self.assertEqual(doc["source_catalogs"], 18)
        self.assertEqual(doc["external_platforms"], 12)
        self.assertEqual(doc["distinct_git_blobs"], 20291)
        self.assertEqual(doc["repeated_blob_excess_occurrences"], 2595)
        self.assertEqual(doc["references_in_repeated_blob_groups"], 5136)
        self.assertEqual(doc["cross_source_shared_blob_groups"], 2)
        self.assertEqual(doc["cross_source_shared_references"], 4)
        self.assertTrue(doc["metadata_only"])
        self.assertFalse(doc["archive_payloads_included"])
        self.assertFalse(doc["independently_verified_games_or_effects"])

    def test_platform_and_source_counts_reconcile(self):
        doc = self.data
        self.assertEqual(sum(x["reference_files"] for x in doc["sources"]), 22886)
        self.assertEqual(sum(x["reference_files"] for x in doc["platforms"]), 22886)
        hamlet = next(x for x in doc["sources"] if x["id"] == "hamlet-switch-cheatdb")
        self.assertEqual((hamlet["reference_files"], hamlet["distinct_git_blobs"]), (5278, 3046))
        self.assertEqual(
            sum(x["repeated_blob_excess_occurrences_within_source"] for x in doc["sources"]), 2593
        )

    def test_cross_source_matches_keep_paths_and_no_bodies(self):
        doc = self.data
        matched = doc["cross_source_exact_blob_matches"]
        self.assertEqual({row["git_blob_sha"] for row in matched}, {
            "bd9c87c6fcebcd47abd376d76cc6628d03c44019",
            "c4b7d0f01961d2ca240e8f3b6a6e0df10274d5fc",
        })
        for row in matched:
            self.assertEqual(row["source_count"], 2)
            self.assertEqual(len(row["references"]), 2)
            self.assertEqual(
                {r["source_id"] for r in row["references"]},
                {"r0ah-vitacheat", "shumnot-vitacheat"},
            )
            for ref in row["references"]:
                self.assertEqual(
                    set(ref), {"source_id", "platform", "path", "git_blob_sha", "bytes"}
                )
                self.assertEqual(ref["platform"], "vita")

    def test_reproducible(self):
        self.assertEqual(self.data, build())


if __name__ == "__main__":
    unittest.main()
