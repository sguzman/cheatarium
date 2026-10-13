#!/usr/bin/env python3
"""Synthetic GameCube/Wii U source-format regressions, no ROMs or network."""
import io
import json
import unittest
import zipfile
from audit_nintendo_bulk_members import (
    analyze_gamecube_zip_contents, analyze_wiiu_xml, analyze_wiiu_json,
)


class NintendoBulkMemberTests(unittest.TestCase):
    def test_region_aliases_count_same_member_bytes_once(self):
        def pack(items):
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, "w") as z:
                for name, body in items:
                    z.writestr(name, body)
            return stream.getvalue()
        a = pack([("GALE01.txt", b"A"), ("GBZE01.txt", b"B")])
        b = pack([("GALE01.txt", b"A"), ("GZLE01.txt", b"C")])
        doc = analyze_gamecube_zip_contents([("NTSC-U Codes.zip", a), ("PAL codes.zip", b)])
        self.assertEqual(doc["total_member_file_occurrences"], 4)
        self.assertEqual(doc["distinct_exact_member_byte_bodies_across_all_regions"], 3)
        self.assertEqual(doc["same_member_path_occurrences_across_regions"], 1)
        self.assertTrue(doc["member_names_hashes_and_code_text_not_exported"])

    def test_rejects_zip_traversal(self):
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as z:
            z.writestr("../outside.txt", "test")
        with self.assertRaises(ValueError):
            analyze_gamecube_zip_contents([("NTSC-U Codes.zip", stream.getvalue())])

    def test_xml_counts_code_authors_and_exact_repeats(self):
        raw = ('<?xml version="1.0"?><codes><entry name="A">'
               '<code>AAAA</code><authors>Author</authors></entry>'
               '<entry name="B"><code>AAAA</code><authors>Author</authors></entry>'
               '</codes>').encode()
        d = analyze_wiiu_xml([("A.xml", raw)])
        self.assertEqual(d["entry_occurrences"], 2)
        self.assertEqual(d["entries_with_author_label"], 2)
        self.assertEqual(d["distinct_exact_nonempty_code_text_bodies"], 1)
        self.assertEqual(d["repeated_nonempty_code_body_occurrences"], 1)
        self.assertFalse(d["original_codes_game_names_credit_strings_or_hashes_exported"])

    def test_mislabelled_utf16_declaration_with_real_utf8_bytes(self):
        original = (b'<?xml version="1.0" encoding="UTF-16"?>'
                    b'<codes><entry><code>AAAA</code><authors>Creator</authors></entry></codes>')
        doc = analyze_wiiu_xml([("misdeclared.xml", original)])
        self.assertEqual(doc["entry_occurrences_with_code_text"], 1)
        self.assertEqual(doc["entries_with_author_label"], 1)

    def test_json_counts_repeated_payloads(self):
        body = json.dumps([{"title": "a", "author": "p",
                            "cafe_code": {"code": "123"}, "ugecko": {"code": ""}}]).encode()
        d = analyze_wiiu_json([("0.json", "a"*40, body),
                               ("1.json", "a"*40, body)])
        self.assertEqual(d["record_occurrences"], 2)
        self.assertEqual(d["distinct_exact_dual_format_code_payload_pairs"], 1)
        self.assertEqual(d["distinct_original_git_file_blobs"], 1)


if __name__ == "__main__":
    unittest.main()
