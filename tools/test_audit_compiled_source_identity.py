#!/usr/bin/env python3
"""ROM-free synthetic tests for bounded compiled cheat source analysis."""
import io
import json
import unittest
import zipfile

from audit_compiled_source_identity import (
    compare_cwcheat, parse_cwcheat, sega_zip_members, summarize_nintendo_exports,
)


class TestCompiledSourceIdentity(unittest.TestCase):
    def test_cwcheat_distinguishes_serials_and_code_blocks(self):
        a = (b"_S GAME-0001\n_G A\n_C0 Name\n_L 0x1 0x2\n"
             b"_C1 Another label\n_L 0x1 0x2\n"
             b"_S GAME-0002\n_G B\n_C0 X\n_L 0x2 0x3\n")
        b = (b"_S GAME-0001\n_C0 Renamed\n_L 0x1 0x2\n"
             b"_S GAME-0003\n_C0 Third\n_L 0x1 0x2\n")
        report = compare_cwcheat(a, b)
        self.assertEqual(report["same_serial_labels_in_both_sources"], 1)
        self.assertEqual(report["a"]["code_bearing_group_occurrences"], 3)
        self.assertEqual(report["a"]["distinct_serial_and_exact_code_block_pairs"], 2)
        self.assertEqual(report["exact_serial_and_code_block_pairs_in_both_sources"], 1)
        self.assertEqual(report["distinct_exact_serial_and_code_block_pairs_in_union"], 3)
        self.assertFalse(report["individual_serials_codes_or_hashes_exported"])

    def test_cwcheat_ignores_commands_not_within_group(self):
        src = parse_cwcheat(b"_L before group\n_S ABC\n_L outside\n_C0 no code\n_S DEF\n_C1 named\n_L ok\n")
        self.assertEqual(len(src["groups"]), 1)
        self.assertEqual(src["source_game_section_occurrences"], 2)

    def test_json_platform_duplicate_exact_codes_without_code_export(self):
        sample = [{"id": "A", "name": "Game", "cheat": [
            {"name": "one", "code": "AA"}, {"name": "two", "code": "AA"},
            {"name": "three", "code": ["XX", "YY"]}]},
            {"id": "A", "name": "Game Edition", "build_id": "rev", "cheat": []}]
        blobs = {key: json.dumps(sample if key == "switch" else []).encode()
                 for key in ("switch", "3ds", "nes", "wiiu")}
        out = summarize_nintendo_exports(blobs)
        self.assertEqual(out["switch"]["repeat_exact_code_text_within_same_id_label"], 1)
        self.assertEqual(out["switch"]["source_game_or_variant_records"], 2)
        self.assertEqual(out["switch"]["distinct_source_game_id_labels"], 1)
        self.assertEqual(out["switch"]["game_records_with_build_id_field"], 1)
        self.assertFalse(out["switch"]["individual_identifiers_titles_or_codes_exported"])

    def test_sega_members_share_exact_bytes_across_differently_named_zips(self):
        def archive(items):
            stream = io.BytesIO()
            with zipfile.ZipFile(stream, "w") as f:
                for name, data in items:
                    f.writestr(name, data)
            return stream.getvalue()
        a = archive([("A.gg", b"\x00\x01"), ("notes.txt", b"Credits: a")])
        b = archive([("B.gg", b"\x00\x01"), ("extra.gg", b"\x00\x03")])
        sources = [("one.zip", "a"*40, a), ("alias.zip", "a"*40, a),
                   ("two.zip", "b"*40, b)]
        r = sega_zip_members(sources)
        self.assertEqual(r["member_file_occurrences_excluding_identical_zip_aliases"], 4)
        self.assertEqual(r["distinct_exact_member_file_bodies"], 3)
        self.assertEqual(r["repeated_member_byte_occurrences"], 1)
        self.assertEqual(r["text_members_containing_credit_marker"], 1)

    def test_rejects_malformed_json_source(self):
        files = {key: b"{}" for key in ("switch", "3ds", "nes", "wiiu")}
        with self.assertRaises(ValueError):
            summarize_nintendo_exports(files)


if __name__ == "__main__":
    unittest.main()
