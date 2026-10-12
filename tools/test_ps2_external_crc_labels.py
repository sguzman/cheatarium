#!/usr/bin/env python3
"""Strict native PS2 PNACH filename identification must never infer compatibility."""
import unittest

from audit_ps2_external_crc_labels import compile_report, parse_pnach_filename


class TestPS2SourceCRCAudit(unittest.TestCase):
    def test_source_native_labels_only(self):
        for path, expected in {
            "patches/6D980D22.pnach": ("6D980D22", None, "crc_only"),
            "cheats/SLUS-20388_339A0B8C.pnach": ("339A0B8C", "SLUS-20388", "serial_crc"),
            "cheats/10B4DF0B - Winning Post 5.pnach": ("10B4DF0B", None, "crc_prefix_title"),
            "cheats/sces-50492_941bb7d9.pnach": None,
            "cheats/SCUS-97203_6441001.pnach": None,
            "cheats/SLUS-20658_9.83E+90.pnach": None,
            "cheats/title/SLUS-20001_ABCDEF12.txt": None,
        }.items():
            label = parse_pnach_filename(path)
            if expected is None:
                self.assertIsNone(label)
            else:
                self.assertEqual(
                    (label["crc"], label["serial"], label["format"]), expected
                )

    def test_same_crc_not_same_game_or_compatible_code(self):
        a = {"id": "source-a", "platform": "ps2", "files": [
            ["cheats/SLUS-20388_339A0B8C.pnach", "a"*40, 3],
            ["cheats/BLES-55555_339A0B8C.pnach", "b"*40, 2],
            ["cheats/not-valid.pnach", "c"*40, 1],
        ]}
        b = {"id": "source-b", "platform": "ps2", "files": [
            ["patches/339A0B8C.pnach", "d"*40, 3],
            ["patches/SLUS-20388_339A0B8C.pnach", "e"*40, 4],
        ]}
        report = compile_report([a, b])
        self.assertEqual(report["source_native_pnach_files"], 5)
        self.assertEqual(report["recognized_crc_filename_occurrences"], 4)
        self.assertEqual(report["unrecognized_crc_filename_occurrences"], 1)
        self.assertEqual(report["distinct_crc_label_strings"], 1)
        self.assertEqual(report["crc_labels_in_two_or_more_sources"], 1)
        self.assertEqual(report["pairwise_source_label_overlap"][0]["shared_crc_label_strings"], 1)
        self.assertEqual(report["pairwise_source_label_overlap"][0]["shared_strict_serial_crc_label_pairs"], 1)
        self.assertFalse(report["filename_labels_are_verified_game_or_rom_build_ids"])
        self.assertFalse(report["independently_verified_games_or_cheat_effects"])
        self.assertFalse(report["cheat_or_patch_payloads_included"])

    def test_empty_and_wrong_platform(self):
        self.assertEqual(compile_report([])["source_native_pnach_files"], 0)
        with self.assertRaises(ValueError):
            compile_report([{"id": "other", "platform": "ps1", "files": []}])


if __name__ == "__main__":
    unittest.main()
