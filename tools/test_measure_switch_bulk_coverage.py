#!/usr/bin/env python3
"""Synthetic regression test of bulk Switch title/build matching."""
import json
import tempfile
import unittest
from pathlib import Path
from measure_switch_bulk_coverage import NX, SHARKIVE, build_report

A = "0100AA001415E000"
B = "0100853015E86000"
C = "0100C2500FC20000"
BUILD1 = "abcdef1234567890"
BUILD2 = "fedcba0987654321"


class TestSwitchBulkCoverage(unittest.TestCase):
    def test_preserves_duplicate_paths_and_nonstandard_builds(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "sources").mkdir()
            sharkive = [f"switch/{A}/{BUILD1}.txt", f"switch/{B}/{BUILD1}.txt",
                        f"switch/{B}/not-a-16-hex-build.txt",
                        "3ds/0004000000007500.txt"]
            nx = [f"{A} Old/atmosphere/contents/{A}/cheats/{BUILD1}.txt",
                  f"_old/Legacy/{A}/cheats/{BUILD1}.txt",
                  f"{C} Title/atmosphere/contents/{C}/cheats/{BUILD2}.txt",
                  "README.md", "LICENSE"]
            for source, paths in ((SHARKIVE, sharkive), (NX, nx)):
                doc = {"id": source, "snapshot_commit": "a" * 40,
                       "files": [{"upstream_path": path} for path in paths]}
                if source == NX:
                    doc["source_native_cheat_txt_files"] = 3
                (root / "sources" / f"{source}.json").write_text(json.dumps(doc))
            report = build_report(root)
            self.assertEqual(report["sources"][NX]["native_cheat_source_files"], 3)
            self.assertEqual(report["sources"][NX]["duplicate_source_paths_for_same_pair"], 1)
            self.assertEqual(report["sources"][SHARKIVE]["noncomparable_native_paths"],
                             [f"switch/{B}/not-a-16-hex-build.txt"])
            self.assertEqual(report["title_id_labels"]["shared"], 1)
            self.assertEqual(report["title_id_labels"]["only_nxcheats"], 1)
            self.assertEqual(report["title_build_id_pairs"]["shared"], 1)
            self.assertEqual(report["title_build_id_pairs"]["only_nxcheats"], 1)
            self.assertEqual(report["title_build_id_pairs"]["combined_distinct"], 3)


if __name__ == "__main__":
    unittest.main()
