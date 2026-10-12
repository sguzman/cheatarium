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
            "ademola-switch-native": 2582,
            "ademola-3ds-native": 257,
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
            "ibnux-switch-cheatdb": 3136,
            "sthetix-switch-mirror": 5861,
            "journeyover-3ds-ctrpf": 628,
            "daowace-pcsx2-consolidated": 5483,
            "xs1l3n7x-pcsx2": 579,
            "ps2-widescreen-staging": 3632,
            "crashgg-ps1-chtdb-delta": 268,
            "deadskullz-nds-formats": 9,
            "saramagrean-psp-cwcheat": 1,
        })
        self.assertEqual(indexes[0]["file_counts"], {"cheat": 4497, "patch": 138})
        self.assertEqual(next(x for x in indexes if x["id"] == "xenia-canary-game-patches")["file_counts"], {"patch": 501})
        self.assertTrue(all(x["rights_status"] == "link_only_rights_review" for x in indexes))
        self.assertEqual(sum(len(x["files"]) for x in indexes), 42483)

    def test_widescreen_staging_is_link_only_and_patch_only(self):
        source = next(x for x in load(__import__("external_bulk").ROOT)
                      if x["id"] == "ps2-widescreen-staging")
        self.assertEqual(len(source["files"]), 3632)
        self.assertEqual(source["file_counts"], {"patch": 3632})
        self.assertEqual(sum(p.startswith("PNACH/") for p, _, _ in source["files"]), 232)
        self.assertEqual(sum(p.startswith("PNACH_WITH_ID/") for p, _, _ in source["files"]), 3400)
        self.assertTrue(all(classify(source["id"], p) == "patch"
                            for p, _, _ in source["files"]))
        found = lookup([source], "0001171A", "ps2", 3)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0]["kind"], "patch")
        self.assertEqual(found[0]["redistribution"], "link_only_rights_review")
        self.assertEqual(found[0]["git_blob_sha"], "8e9f2a29fdec4482c17b9c50e51ba95337a93e3a")
        self.assertTrue(found[0]["upstream_url"].startswith(
            "https://github.com/PS2-Widescreen/pnach-DB/blob/91902dd63"))

    def test_ps1_fork_tracks_only_byte_distinct_delta(self):
        indexes = load(__import__("external_bulk").ROOT)
        parent = next(x for x in indexes if x["id"] == "duckstation-chtdb")
        fork = next(x for x in indexes if x["id"] == "crashgg-ps1-chtdb-delta")
        known = {sha for _, sha, _ in parent["files"]}
        self.assertEqual(len(fork["files"]), 268)
        self.assertEqual(fork["file_counts"], {"cheat": 263, "patch": 5})
        self.assertTrue(all(sha not in known for _, sha, _ in fork["files"]))
        result = lookup([fork], "SCES-00242", "ps1", 4)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["kind"], "cheat")
        self.assertEqual(result[0]["redistribution"], "link_only_rights_review")

    def test_two_bulky_compiled_database_sources_are_metadata_only(self):
        indexes = load(__import__("external_bulk").ROOT)
        ds = next(x for x in indexes if x["id"] == "deadskullz-nds-formats")
        psp = next(x for x in indexes if x["id"] == "saramagrean-psp-cwcheat")
        self.assertEqual((len(ds["files"]), len(psp["files"])), (9, 1))
        self.assertEqual((len({r[1] for r in ds["files"]}),
                          len({r[1] for r in psp["files"]})), (8, 1))
        self.assertEqual(ds["file_counts"], {"cheat": 9})
        self.assertEqual(psp["file_counts"], {"cheat": 1})
        self.assertTrue(all(classify(ds["id"], p) == "cheat"
                            for p, _, _ in ds["files"]))
        self.assertTrue(all(classify(psp["id"], p) == "cheat"
                            for p, _, _ in psp["files"]))
        self.assertEqual(len(lookup([ds], "usrcheat.dat", "ds", 5)), 1)
        self.assertEqual(len(lookup([psp], "cheat.db", "psp", 5)), 1)
        self.assertTrue(all(s["rights_status"] == "link_only_rights_review"
                            for s in (ds, psp)))

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
        index = next(x for x in load(__import__("external_bulk").ROOT)
                     if x["id"] == "xenia-canary-game-patches")
        matches = lookup([index], "painkiller", "xbox-360", 4)
        self.assertTrue(matches)
        self.assertTrue(all(m["redistribution"] == "link_only_rights_review" for m in matches))
        self.assertTrue(all(m["upstream_url"].startswith(index["repository"] + "/blob/") for m in matches))
        self.assertEqual(lookup([index], "painkiller", "ps1", 4), [])
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "SLUS-", "ps2", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "PCSA", "vita", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "0100000000010000", "switch", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "Mario Kart", "wii", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "0004000000188100", "3ds", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), ".ips", "switch", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "USERLIST", "ps3", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "0100000000010000", "switch", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "00040000000E7300", "3ds", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "cheat.db", "psp", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "usrcheat.dat", "ds", 5))
        self.assertTrue(lookup(load(__import__("external_bulk").ROOT), "0100000000010000", "switch", 5))

if __name__ == "__main__":
    unittest.main()
