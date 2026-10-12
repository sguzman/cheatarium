#!/usr/bin/env python3
"""Offline regression tests for historical metadata-only references."""
import unittest

from export_historic_libretro import candidate_rows, render_catalog
from audit_historic_libretro import (
    HIST_REV, HIST_TREE, compile_report,
)


class HistoricExternalTests(unittest.TestCase):
    def test_only_bytes_absent_from_every_preserved_source_are_selected(self):
        historic = [
            ("DS/A.cht", "a" * 40, 10),
            ("DS/B.cht", "b" * 40, 20),
            ("DS/C.cht", "c" * 40, 30),
            ("DS/D.cht", "d" * 40, 40),
            ("DS/E.cht", "d" * 40, 40),
        ]
        baseline = [("DS/modern.cht", "a" * 40, 10)]
        archive = [("c" * 40, "source", "archive/source/C")]
        rows = candidate_rows(historic, baseline, {"b" * 40}, archive)
        self.assertEqual(rows, [
            ("DS/D.cht", "d" * 40, 40),
            ("DS/E.cht", "d" * 40, 40),
        ])

    def test_catalog_is_rights_blocked_and_preserves_rumble_signal(self):
        historic = [
            ("DS/A (RetroArch Rumbles).cht", "a" * 40, 1),
            ("DS/B (RetroArch Cheats).cht", "b" * 40, 2),
        ]
        audit = compile_report(historic, [], set(), [])
        catalog = render_catalog(candidate_rows(historic, [], set(), []),
                                 audit)
        self.assertEqual(catalog["rumble_filename_signal_occurrences"], 1)
        self.assertEqual(catalog["distinct_git_blob_sha1"], 2)
        self.assertFalse(catalog["code_payloads_included"])
        self.assertFalse(catalog["archived"])
        self.assertEqual(catalog["rights_status"], "link_only_rights_review")
        self.assertEqual(catalog["snapshot_commit"], HIST_REV)
        self.assertEqual(catalog["source_cheat_tree_sha"], HIST_TREE)
        self.assertIn("%20", catalog["files"][0]["upstream_url"])

    def test_stale_audit_cannot_export_misleading_counts(self):
        audit = compile_report([("DS/A.cht", "a" * 40, 1)], [], set(), [])
        with self.assertRaises(ValueError):
            render_catalog([], audit)


if __name__ == "__main__":
    unittest.main()
