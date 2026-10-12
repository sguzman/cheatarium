#!/usr/bin/env python3
"""ROM-free exact-blob source acquisition overlap regression tests."""
import tempfile
import json
import unittest
from pathlib import Path

from audit_external_archive_overlap import compile_report, load_archived


def source(source_id, platform, rows):
    return {"id": source_id, "platform": platform, "files": rows}


class ExternalArchiveOverlapTest(unittest.TestCase):
    def test_distinguishes_occurrences_from_distinct_blobs(self):
        a, b, c = ["a"*40, "b"*40, "c"*40]
        archived = [(a, "import-a", "archive/import-a/file.txt"),
                    (a, "import-b", "archive/import-b/file.txt"),
                    (c, "import-a", "archive/import-a/other.txt")]
        external = [
            source("source-x", "switch", [["one.txt", a, 3], ["two.txt", a, 3],
                                          ["fresh.txt", b, 9]]),
            source("source-y", "ps2", [["unmatched.pnach", b, 9]]),
        ]
        report = compile_report(archived, external)
        self.assertEqual(report["archive_manifest_file_occurrences"], 3)
        self.assertEqual(report["archive_distinct_git_blobs"], 2)
        self.assertEqual(report["external_reference_files"], 4)
        self.assertEqual(report["external_distinct_git_blobs"], 2)
        self.assertEqual(report["external_references_matching_archived_bytes"], 2)
        self.assertEqual(report["external_distinct_git_blobs_matching_archive"], 1)
        self.assertEqual(report["external_references_without_archived_byte_match"], 2)
        self.assertEqual(report["external_distinct_git_blobs_without_archive_byte_match"], 1)
        self.assertEqual(report["sources"][0]["exact_byte_match_examples"][0]["archived_source_id"], "import-a")
        self.assertFalse(report["establishes_external_redistribution_rights"])
        self.assertFalse(report["proves_game_or_cheat_equivalence"])
        self.assertEqual(sum(x["reference_files"] for x in report["platforms"]), 4)

    def test_archive_loader_is_manifest_scoped_and_checks_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source_path = root / "sources"
            source_path.mkdir()
            (source_path / "candidates.json").write_text('{"candidates":[]}', "utf-8")
            (source_path / "legit.json").write_text(json.dumps({
                "id": "legit", "files": [{"git_blob_sha": "f"*40,
                    "upstream_path": "foo.txt", "archive_path": "archive/legit/foo.txt"}]}), "utf-8")
            self.assertEqual(load_archived(root), [
                ("f"*40, "legit", "archive/legit/foo.txt")
            ])
            (source_path / "legit.json").write_text(json.dumps({
                "id": "legit", "files": [{"git_blob_sha": "f"*40,
                    "archive_path": "archive/other/foo.txt"}]}), "utf-8")
            with self.assertRaises(ValueError):
                load_archived(root)

    def test_invalid_git_blob_hash_rejected(self):
        with self.assertRaises(ValueError):
            compile_report([("not-a-sha", "s", "archive/s/x")], [])
        with self.assertRaises(ValueError):
            compile_report([], [source("s", "ps1", [["foo.cht", "nothash", 20]])])


if __name__ == "__main__":
    unittest.main()
