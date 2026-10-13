#!/usr/bin/env python3
"""Offline read-only Wii native group selection and version safety tests."""
import unittest

from query_wii_gecko_local import parse_gecko, query, source_identity


def sample_entry(disc, rev=None, variant="base", source="wii-accf-gecko-native"):
    stem = disc + (rev or "") + (("_" + variant) if variant != "base" else "")
    return {
        **source_identity("codes/" + stem + ".ini"),
        "source_id": source, "game_family_title_hint": {
            "wii-accf-gecko-native": "Animal Crossing: City Folk / Deluxe",
            "wii-metroid-prime-gecko-native": "Metroid Prime / Trilogy",
        }.get(source, "Unmapped synthetic family"),
        "source_compatibility_warning": "Source file revision must be selected",
        "local_path": "wii/" + source + "/codes/" + stem + ".ini",
        "upstream_original_url": "https://github.com/test/cheat/blob/" + "a"*40 + "/codes/" + stem + ".ini",
        "original_git_blob_sha1": "b"*40, "source_rights": "link_only_rights_review",
        "groups": parse_gecko(b"[Gecko]\n$Classic Controller\n*By Original Author\n"
                              b"04001234 00112233\n$Other\n12345678 ABCDEF09\n"
                              b"[Gecko_Enabled]\n$Classic Controller\n",
                              "codes/" + stem + ".ini"),
    }


class WiiLocalQueryTests(unittest.TestCase):
    def test_preserves_explicit_source_revision(self):
        self.assertEqual(source_identity("gecko/RUUE01v0.txt")["source_revision_token"], "v0")
        self.assertEqual(source_identity("gecko/RUUE01v1.txt")["source_revision_token"], "v1")
        self.assertEqual(source_identity("codes/R3ME01_mp1.ini")["source_variant_token"], "mp1")
        self.assertEqual(source_identity("codes/R3ME01.ini")["source_variant_token"], "base")
        with self.assertRaises(ValueError):
            source_identity("codes/RUUE01-v0.ini")

    def test_metadata_query_never_emits_code_text_or_auto_selects_revision(self):
        entries = [sample_entry("RUUE01", "v0"), sample_entry("RUUE01", "v1")]
        result = query(entries, game_id="RUUE01")
        self.assertEqual(result["file_matches"], 2)
        self.assertTrue(result["ambiguous_source_variants"])
        self.assertFalse(result["original_code_text_included"])
        self.assertFalse(result["gameplay_effect_verified"])
        self.assertNotIn("00112233", str(result))
        self.assertEqual({r["source_revision_token"] for r in result["records"]}, {"v0", "v1"})

    def test_original_code_visible_only_with_exact_revision_and_index(self):
        entries = [sample_entry("RUUE01", "v0"), sample_entry("RUUE01", "v1")]
        with self.assertRaisesRegex(ValueError, "Ambiguous"):
            query(entries, game_id="RUUE01", group_index=1, show_code=True)
        with self.assertRaisesRegex(ValueError, "requires"):
            query(entries, game_id="RUUE01", group_index=1)
        d = query(entries, game_id="RUUE01", revision="0", group_index=1, show_code=True)
        self.assertEqual(d["file_matches"], 1)
        self.assertEqual(d["records"][0]["cheat_groups"][0]["code_lines"],
                         ["04001234 00112233"])
        self.assertTrue(d["records"][0]["cheat_groups"][0]["source_enabled"])
        self.assertFalse(d["code_executed_or_enabled"])

    def test_trilogy_variant_requires_selection_before_code_view(self):
        entries = [sample_entry("R3ME01"), sample_entry("R3ME01", variant="mp1")]
        with self.assertRaisesRegex(ValueError, "Ambiguous"):
            query(entries, game_id="R3ME01", group_index=1, show_code=True)
        d = query(entries, game_id="R3ME01", variant="mp1", group_index=1, show_code=True)
        self.assertEqual(d["records"][0]["source_variant_token"], "mp1")

    def test_group_title_cannot_silently_pick_incompatible_revision(self):
        entries = [sample_entry("RUUE01", "v0"), sample_entry("RUUE01", "v1")]
        entries[1]["groups"][0]["title"] = "Entirely Different Header"
        with self.assertRaisesRegex(ValueError, "Ambiguous"):
            query(entries, game_id="RUUE01", title="Classic", group_index=1, show_code=True)
        by_family = query(entries, game_title="city folk")
        self.assertEqual(by_family["file_matches"], 2)
        self.assertIn("revision", by_family["records"][0]["source_compatibility_warning"])

    def test_title_search_and_pagination(self):
        entries = [sample_entry("R3ME01"), sample_entry("R3OE01"), sample_entry("R2JJAF")]
        a = query(entries, title="Classic", limit=1, offset=1)
        self.assertEqual(a["file_matches"], 3)
        self.assertEqual(a["returned"], 1)
        self.assertTrue(a["has_more"])
        b = query(entries, title="By Original Author")
        self.assertEqual(b["file_matches"], 3)
        with self.assertRaises(ValueError):
            query(entries, game_id="NOT A REAL ID")

    def test_gecko_enabled_is_not_a_cheat_group(self):
        g = parse_gecko(b"[Gecko]\n$Name\n01234567 89ABCDEF\n"
                        b"[Gecko_Enabled]\n$Name\n", "A12345.ini")
        self.assertEqual(len(g), 1)
        self.assertTrue(g[0]["source_enabled"])

    def test_invalid_and_orphan_source_codes_fail_closed(self):
        with self.assertRaisesRegex(ValueError, "Orphan"):
            parse_gecko(b"[Gecko]\n00000000 00000001\n$A\n", "ABCDEF.ini")
        with self.assertRaisesRegex(ValueError, "Unrecognized"):
            parse_gecko(b"$A\nNOT A CODE\n", "ABCDEF.txt")
        with self.assertRaisesRegex(ValueError, "Missing"):
            parse_gecko(b"[Something_Else]\n$A\n00000000 00000001\n", "ABCDEF.ini")


if __name__ == "__main__":
    unittest.main()
