#!/usr/bin/env python3
"""Synthetic ROM-free tests of native Switch ID comparison."""
import unittest
from audit_switch_external_title_build import compare, parsed_external

T1 = "0100000000000001"
T2 = "0100000000000002"
T3 = "0100000000000003"
B1 = "abcdef0123456789"
B2 = "9876543210fedcba"


def entry(id, paths):
    return {"id": id, "platform": "switch", "snapshot_commit": "a"*40,
            "rights_status": "link_only_rights_review",
            "files": [[p, "a"*40, 10] for p in paths]}


class SwitchExternalCoverageTests(unittest.TestCase):
    def setUp(self):
        self.archived = {
            "sharkive": {"id": "sharkive", "files": [
                {"upstream_path": f"switch/{T1}/{B1}.txt"},
            ]},
            "bad1dea-nxcheats-switch": {
                "id": "bad1dea-nxcheats-switch",
                "source_native_cheat_txt_files": 1,
                "files": [{"upstream_path": f"{T2} Legacy/cheats/{B2}.txt"}],
            },
        }
        self.external = [
            entry("ademola-switch-native", [
                f"Titles/{T1}/cheats/{B1}.txt",
                f"Titles/{T3}/cheats/{B2}.txt",
                "Game/60fps/1.0.pchtxt",
            ]),
            entry("ibnux-switch-cheatdb", [
                f"atmosphere/titles/{T3}/cheats/{B2}.txt",
            ]),
            entry("hamlet-switch-cheatdb", [
                f"cheats/{T1}.json", f"cheats_gbatemp/{T1}.json",
                f"cheats/{T3}.json", f"cheats_gfx/{T2}.json",
            ]),
            entry("sthetix-switch-mirror", [
                f"cheats/{T3}.json", f"cheats_gfx/{T2}.json",
            ]),
            entry("rogamer97-asm-switch", [
                "Switch/Sample/Cheat Code.md",
                "Switch/Sample/FPS.pchtxt",
            ]),
        ]

    def test_native_pairs_and_mirror_title_only_are_distinct(self):
        report = compare(self.archived, self.external)
        s = report["summary"]
        self.assertEqual(s["imported_distinct_native_title_build_pairs"], 2)
        self.assertEqual(s["external_distinct_native_title_build_pairs"], 2)
        self.assertEqual(s["native_pairs_shared_by_archive_and_external"], 1)
        self.assertEqual(s["native_pairs_only_in_external_sources"], 1)
        self.assertEqual(s["combined_distinct_native_title_build_pairs"], 3)
        self.assertEqual(s["mirror_distinct_title_only_json_labels"], 2)
        self.assertEqual(s["mirror_title_labels_without_archived_native_title"], 1)
        self.assertFalse(report["rights_unresolved_external_payloads_imported"])
        self.assertFalse(report["json_mirror_has_build_id_evidence"])
        self.assertFalse(report["proves_distinct_games_or_new_cheats"])
        rows = {r["id"]: r for r in report["external_sources"]}
        self.assertEqual(rows["hamlet-switch-cheatdb"]["title_only_json_file_occurrences"], 3)
        self.assertEqual(rows["hamlet-switch-cheatdb"]["patch_file_occurrences_not_assumed_cheats"], 1)
        self.assertEqual(rows["rogamer97-asm-switch"]["other_cheat_documents_without_exact_native_ids"], 1)
        self.assertEqual(rows["ademola-switch-native"]["native_pairs_also_in_imported_sources"], 1)
        self.assertEqual(rows["ibnux-switch-cheatdb"]["native_pairs_not_in_imported_sources"], 1)

    def test_unexpected_native_path_rejected(self):
        with self.assertRaises(ValueError):
            parsed_external("ibnux-switch-cheatdb", [["strange.txt", "a"*40, 5]])

    def test_unreviewed_source_rejected(self):
        with self.assertRaises(ValueError):
            compare(self.archived, self.external + [entry("unknown-switch-mirror", [])])


if __name__ == "__main__":
    unittest.main()
