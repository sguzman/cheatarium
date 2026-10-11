#!/usr/bin/env python3
"""Safety and identity regression tests for link-only bulk reference inventories."""
import unittest
from external_bulk import SOURCES, classify, load, lookup, source_link, validate

class TestExternalBulk(unittest.TestCase):
    def test_pinned_inventories(self):
        indexes = load(__import__("external_bulk").ROOT)
        self.assertEqual([len(x["files"]) for x in indexes], [4635, 501])
        self.assertEqual(indexes[0]["file_counts"], {"cheat": 4497, "patch": 138})
        self.assertEqual(indexes[1]["file_counts"], {"patch": 501})
        self.assertTrue(all(x["rights_status"] == "link_only_rights_review" for x in indexes))

    def test_rejects_payload_and_traversal(self):
        indexes = load(__import__("external_bulk").ROOT)
        for key, value in [("payload", "THIS IS NOT METADATA"), ("rights_status", "imported")]:
            broken = dict(indexes[0])
            broken[key] = value
            with self.assertRaises(ValueError):
                validate("duckstation-chtdb", broken)
        broken = dict(indexes[0])
        broken["files"] = [["../bad.cht", "0" * 40, 100]]
        with self.assertRaises(ValueError):
            validate("duckstation-chtdb", broken)

    def test_lookup_is_link_only(self):
        index = load(__import__("external_bulk").ROOT)[1]
        matches = lookup([index], "painkiller", "xbox-360", 4)
        self.assertTrue(matches)
        self.assertTrue(all(m["redistribution"] == "link_only_rights_review" for m in matches))
        self.assertTrue(all(m["upstream_url"].startswith(index["repository"] + "/blob/") for m in matches))
        self.assertEqual(lookup([index], "painkiller", "ps1", 4), [])

if __name__ == "__main__":
    unittest.main()
