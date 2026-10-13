#!/usr/bin/env python3
"""Synthetic MAME DAT/ZIP label comparison tests; no source downloads."""
import io
import unittest
import zipfile

from audit_mame_dat_xml_labels import parse_dat, compare_dat_xml


def zip_names(paths):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as f:
        for p in paths:
            f.writestr(p, "<mamecheat/>")
    return buffer.getvalue()


class TestMameDatLabels(unittest.TestCase):
    def test_driver_labels_and_malformed_rows(self):
        data = (
            b"; comment\n"
            b":_command:00000001\n"
            b":pacman:00000000:12345678:0A:FFFFFFFF:Invincibility\n"
            b":pacman:00000000:12345678:0A:FFFFFFFF:Invincibility\n"
            b":galaga:00000000:12345678:0A:FFFFFFFF:Infinite\n"
            b":bad:NOTHEX:0000:0000:0000:Broken\n"
        )
        result = parse_dat(data)
        self.assertEqual(result["native_cheat_dat_record_occurrences"], 3)
        self.assertEqual(result["distinct_native_driver_labels"], 2)
        self.assertEqual(result["duplicate_native_record_text_occurrences"], 1)
        self.assertEqual(result["internal_command_lines"], 1)
        self.assertEqual(result["unparsed_colon_line_occurrences"], 1)

    def test_native_driver_name_overlap_does_not_verify_effect(self):
        data = (
            b":pacman:00000000:12345678:0A:FFFFFFFF:Cheat\n"
            b":galaga:00000000:12345678:0A:FFFFFFFF:Cheat\n"
        )
        doc = compare_dat_xml(data, zip_names(["folder/pacman.xml", "folder/1941.xml"]))
        self.assertEqual(doc["distinct_driver_labels_in_both_dat_and_xml"], 1)
        self.assertEqual(doc["distinct_driver_labels_only_in_dat"], 1)
        self.assertEqual(doc["distinct_driver_labels_only_in_xml"], 1)
        self.assertFalse(doc["exact_game_release_or_code_equivalence_verified"])

    def test_rejects_zip_traversal_and_wrong_member_type(self):
        for path in ("../escape.xml", "not-cheat.txt"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                compare_dat_xml(b"", zip_names([path]))


if __name__ == "__main__":
    unittest.main()
