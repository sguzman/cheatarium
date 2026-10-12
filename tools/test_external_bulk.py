#!/usr/bin/env python3
"""Safety and identity regression tests for link-only bulk reference inventories."""
import unittest
from external_bulk import SOURCES, classify, load, lookup, source_link, validate

class TestExternalBulk(unittest.TestCase):
    def test_pinned_inventories(self):
        indexes = load(__import__("external_bulk").ROOT)
        self.assertEqual({x["id"]: len(x["files"]) for x in indexes}, {
            "duckstation-chtdb": 4635,
            "xenia-canary-game-patches": 501,
            "pcsx2-official-patches": 4710,
            "r0ah-vitacheat": 677,
            "shumnot-vitacheat": 119,
            "bucanero-dreamcast-cheats": 387,
            "ademola-switch-native": 2516,
            "rogamer97-asm-3ds": 53,
            "rogamer97-asm-ds": 5,
            "rogamer97-asm-gamecube": 30,
            "rogamer97-asm-wii": 306,
            "rogamer97-asm-switch": 418,
            "dospamu-rpcs3-artemis-patches": 2687,
            "chidreams-rpcs3-patches": 239,
            "apelfactory-ppsspp-cheatdb": 1,
            "real-f00-usrcheat-ds": 1,
            "hamlet-switch-cheatdb": 5278,
        })
        self.assertEqual(indexes[0]["file_counts"], {"cheat": 4497, "patch": 138})
        self.assertEqual(indexes[1]["file_counts"], {"patch": 501})
        self.assertTrue(all(x["rights_status"] == "link_only_rights_review" for x in indexes))
        self.assertEqual(sum(len(x["files"]) for x in indexes), 22563)

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
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "SLUS-", "ps2", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "PCSA", "vita", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "0100000000010000", "switch", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "Mario Kart", "wii", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "USERLIST", "ps3", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "cheat.db", "psp", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "usrcheat.dat", "ds", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "0100000000010000", "switch", 5))

if __name__ == "__main__":
    unittest.main()
