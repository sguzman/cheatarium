#!/usr/bin/env python3
"""Metadata-only external distribution snapshot tests."""
import gzip
import json
import unittest
from build_external_distribution import render, FORMAT, STATUS


class TestExternalDistribution(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = render()
        cls.catalog = json.loads(cls.files["external/catalog.json"])

    def test_full_pinned_collection(self):
        self.assertEqual(self.catalog["reference_files"], 43409)
        self.assertEqual(len(self.catalog["platforms"]), 26)
        self.assertEqual({x["platform"]: x["source_files"] for x in self.catalog["platforms"]},
                         {"3ds": 938, "arcade-mame": 2, "arcade-fba": 98, "n64": 1, "arcade-fbneo": 32, "arcade-naomi": 9,
                          "multi-retroarch": 22, "nes": 8, "gba": 2, "snes": 1, "dreamcast": 399, "ds": 15, "gamecube": 30,
                          "ps1": 4903, "ps2": 14452, "ps3": 2926, "psp": 2,
                          "switch": 17275, "vita": 796, "wii": 306, "xbox-360": 501})
        self.assertTrue(self.catalog["metadata_only"])
        self.assertFalse(self.catalog["archive_included"])

    def test_only_metadata_fields_exported(self):
        for platform in self.catalog["platforms"]:
            rel = platform["artifact"]
            raw = gzip.decompress(self.files[rel])
            body = json.loads(raw)
            self.assertEqual(body["format"], FORMAT)
            self.assertEqual(body["rights_status"], STATUS)
            self.assertTrue(body["metadata_only"])
            self.assertFalse(body["gameplay_verified"])
            self.assertEqual(len(body["records"]), platform["source_files"])
            for record in body["records"]:
                self.assertEqual(set(record), {
                    "source_id", "kind", "path", "git_blob_sha", "bytes",
                    "repository", "revision", "upstream_url", "rights_status",
                })
                self.assertEqual(record["rights_status"], STATUS)
                self.assertTrue(record["upstream_url"].startswith(
                    record["repository"] + "/blob/" + record["revision"] + "/"))
                self.assertRegex(record["git_blob_sha"], "^[0-9a-f]{40}$")

    def test_gzip_is_reproducible(self):
        self.assertEqual(self.files, render())
        self.assertTrue(self.files["external/ps2.json.gz"].startswith(b"\x1f\x8b"))


if __name__ == "__main__":
    unittest.main()
