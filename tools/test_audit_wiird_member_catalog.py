#!/usr/bin/env python3
"""Synthetic file-only GameCube cheat member provenance tests."""
import io
import unittest
import zipfile

from audit_wiird_member_catalog import index_archive, collect


def pack(items):
    file = io.BytesIO()
    with zipfile.ZipFile(file, "w") as z:
        for path, payload in items:
            z.writestr(path, payload)
    return file.getvalue()


class TestWiiRDMemberCatalog(unittest.TestCase):
    def setUp(self):
        self.s = {"repository": "https://github.com/owner/cheats",
                  "snapshot_commit": "a"*40}
        self.row = ["NTSC-U Codes.zip", "b"*40, 1]

    def test_nested_title_names_and_links_without_code_bytes(self):
        out = index_archive(self.s, self.row,
                            pack([("GC/GZLE01.txt", b"DEADBEEF")]))
        self.assertEqual(out[0]["source_filename_title_hint"], "GZLE01")
        self.assertEqual(out[0]["region"], "NTSC-U")
        self.assertEqual(out[0]["native_zip_member_path"], "GC/GZLE01.txt")
        self.assertIn("/blob/" + "a"*40 + "/", out[0]["original_container_url"])
        self.assertNotIn("DEADBEEF", str(out))
        self.assertNotIn("member_sha", str(out))

    def test_rejects_unsafe_member_paths(self):
        for filename in ("../bad.txt", "/absolute.txt", "game.exe"):
            with self.subTest(filename=filename), self.assertRaises(ValueError):
                index_archive(self.s, self.row, pack([(filename, b"A")]))

    def test_aggregate_metadata_only(self):
        report = collect([(self.s, self.row, pack([("a.txt", b"cheat")]))])
        self.assertEqual(report["reference_count"], 1)
        self.assertFalse(report["cheat_bodies_included"])
        self.assertFalse(report["game_identifiers_or_revisions_confirmed"])


if __name__ == "__main__":
    unittest.main()
