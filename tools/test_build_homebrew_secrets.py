#!/usr/bin/env python3
"""Offline regression tests for public-domain built-in secret indexing."""
import unittest
from build_homebrew_secrets import parse_original, search


class HomebrewSecretTests(unittest.TestCase):
    def test_trailing_commas_and_literal_quoted_delimiters(self):
        s = 'const jsonCheatEntries = [{"title": "A, }]", "a": [1, 2,],},];'
        self.assertEqual(parse_original(s), [{"title": "A, }]", "a": [1, 2]}])
        for bad in ["alert('run')", s + "\nalert('run')",
                    "const jsonCheatEntries = [];console.log(1)"]:
            with self.assertRaises(ValueError):
                parse_original(bad)

    def test_search_is_platform_scoped_read_only(self):
        rows = {"records": [
            {"game_title_source_claim": "Canyon Racer", "developer_source_claim": "bbbbbr",
             "effect_source_claim": "99 Lives", "input_sequence_source_claim": "LEFT",
             "platforms": ["gb"]},
            {"game_title_source_claim": "Test", "developer_source_claim": "Somebody",
             "effect_source_claim": "Sound Test", "input_sequence_source_claim": "START",
             "platforms": ["nes"]},
        ]}
        self.assertEqual(len(search(rows, "lives", "gb")), 1)
        self.assertEqual(len(search(rows, "lives", "nes")), 0)
        self.assertEqual(len(search(rows, "somebody")), 1)


if __name__ == "__main__":
    unittest.main()
