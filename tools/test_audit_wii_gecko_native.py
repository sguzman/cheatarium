#!/usr/bin/env python3
"""Code-free synthetic Gecko syntax parser regressions."""
import unittest
from audit_wii_gecko_native import inspect_text


class WiiGeckoFormatTests(unittest.TestCase):
    def test_native_dolphin_ini_excludes_enabled_selectors(self):
        sample = (b"[Gecko]\n$Classic Controller\n* creator\n"
                  b"C23BD850 0000003B\n80000000 00000000\n"
                  b"$GameCube Controller\n04001234 01234567\n"
                  b"[Gecko_Enabled]\n$Classic Controller\n$GameCube Controller\n")
        c = inspect_text(sample, "R3ME01_mp1.ini")
        self.assertEqual(c["declared_cheat_header_occurrences"], 2)
        self.assertEqual(c["cheat_headers_with_hex_code_pairs"], 2)
        self.assertEqual(c["eight_hex_word_pair_line_occurrences"], 3)
        self.assertEqual(c["source_enabled_selector_name_occurrences"], 2)
        self.assertEqual(c["source_comment_line_occurrences"], 1)
        self.assertEqual(c["unrecognized_active_section_lines"], 0)

    def test_native_revision_specific_plain_txt_has_one_group(self):
        c = inspect_text(
            b"$SDHC Support\n* RUUE01 Rev 0 only, Rev 1 uses different address\n"
            b"C222412C 00000004\n546302D7 41820014\n",
            "RUUE01v0.txt")
        self.assertEqual(c["declared_cheat_header_occurrences"], 1)
        self.assertEqual(c["cheat_headers_with_hex_code_pairs"], 1)
        self.assertEqual(c["eight_hex_word_pair_line_occurrences"], 2)

    def test_missing_code_does_not_become_verified_effect(self):
        c = inspect_text(b"[Gecko]\n$Empty\n* comment\n"
                         b"$Active\n01234567 ABCDEF09\n",
                         "ABCDEF.ini")
        self.assertEqual(c["declared_cheat_header_occurrences"], 2)
        self.assertEqual(c["cheat_headers_with_hex_code_pairs"], 1)

    def test_non_native_format_rejected(self):
        with self.assertRaises(ValueError):
            inspect_text(b"cheat data", "rom.iso")


if __name__ == "__main__":
    unittest.main()
