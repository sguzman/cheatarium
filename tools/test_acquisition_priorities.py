#!/usr/bin/env python3
"""Offline regression coverage for byte-unique acquisition priorities."""
import unittest

from build_acquisition_priorities import compile_report


def source(name, platform, rows, rights="link_only_rights_review"):
    return {"id": name, "platform": platform, "rights_status": rights, "files": rows}


def kind(_source, path):
    return "cheat" if path.endswith(".cht") else "patch"


class AcquisitionPriorityTests(unittest.TestCase):
    def test_mirrors_and_patch_files_do_not_inflate_new_cheats(self):
        a, b, c, d = ("a" * 40, "b" * 40, "c" * 40, "d" * 40)
        archive = [(a, "archived", "archive/archived/a")]
        rows = [
            source("one", "ds", [["old.cht", a, 5], ["shared.cht", b, 7],
                                 ["exclusive.cht", c, 8], ["renamed.cht", c, 8]]),
            source("two", "ps2", [["shared.patch", b, 7], ["only.patch", d, 9]]),
        ]
        result = compile_report(archive, rows, kind)
        self.assertEqual(result["source_file_occurrences"], 6)
        self.assertEqual(result["all_external_distinct_blob_sha1"], 4)
        self.assertEqual(result["distinct_blobs_matching_archived_bytes"], 1)
        self.assertEqual(result["distinct_blobs_absent_from_archive"], 3)
        self.assertEqual(result["distinct_cheat_labeled_blobs_absent_from_archive"], 2)
        self.assertEqual(result["distinct_patch_labeled_blobs_absent_from_archive"], 2)
        self.assertEqual(result["blobs_labeled_both_cheat_and_patch"], 1)
        self.assertEqual(result["exclusive_unarchived_blobs_in_one_source"], 2)
        self.assertEqual(result["shared_unarchived_blobs_in_multiple_sources"], 1)
        first, second = result["sources"]
        self.assertEqual(first["source_id"], "one")
        self.assertEqual(first["exclusive_unarchived_cheat_labeled_blobs"], 1)
        self.assertEqual(first["exclusive_unarchived_blob_sha1"], 1)
        self.assertEqual(first["shared_with_other_external_sources_unarchived_blobs"], 1)
        self.assertEqual(len(first["sample_exclusive_cheat_source_paths"]), 2)
        self.assertEqual(second["source_id"], "two")
        self.assertEqual(second["exclusive_unarchived_cheat_labeled_blobs"], 0)
        self.assertEqual(second["exclusive_unarchived_blob_sha1"], 1)
        self.assertFalse(result["source_rights_cleared_for_import"])
        self.assertFalse(result["unique_games_or_effects_established"])

    def test_rejects_duplicate_paths_sources_sizes_and_invalid_rights(self):
        s = source("one", "ds", [["A.cht", "a" * 40, 1]])
        invalid = [
            [s, s],
            [source("one", "ds", [["A.cht", "a" * 40, 1],
                                  ["A.cht", "b" * 40, 2]])],
            [source("one", "ds", [["A.cht", "a" * 40, 1]], "cleared")],
            [source("one", "ds", [["A.cht", "nonsense", 1]])],
            [source("one", "ds", [["A.cht", "a" * 40, True]])],
            [source("one", "ds", [["A.cht", "a" * 40, 1]]),
             source("two", "gba", [["B.cht", "a" * 40, 2]])],
        ]
        for rows in invalid:
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                compile_report([], rows, kind)

    def test_empty_reference_sets_do_not_create_fake_gaps(self):
        result = compile_report([], [], kind)
        self.assertEqual(result["distinct_blobs_absent_from_archive"], 0)
        self.assertEqual(result["source_file_occurrences"], 0)
        self.assertEqual(result["platforms"], [])
        self.assertEqual(result["sources"], [])

    def test_byte_identity_not_filename_identity(self):
        result = compile_report([], [
            source("one", "ds", [["same.cht", "a" * 40, 2]]),
            source("two", "ds", [["same.cht", "b" * 40, 2]]),
        ], kind)
        self.assertEqual(result["exclusive_unarchived_blobs_in_one_source"], 2)
        self.assertEqual(result["distinct_blobs_absent_from_archive"], 2)


if __name__ == "__main__":
    unittest.main()
