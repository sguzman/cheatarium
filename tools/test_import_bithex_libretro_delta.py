#!/usr/bin/env python3
"""ROM-free pinned Libretro-fork delta-selection regression checks."""
import unittest

from import_bithex_libretro_delta import blob, selected_delta, tree_rows


class BithexDeltaTests(unittest.TestCase):
    def test_git_blob(self):
        self.assertEqual(blob(b"hello\n"), "ce013625030ba8dba906f756967f9e9ca394464a")

    def test_new_blob_filter_not_filename_filter(self):
        rows = [
            ("cht/MSX/a.cht", "a"*40),
            ("cht/MSX/b.cht", "b"*40),
            ("cht/MSX/c.cht", "b"*40),
            ("cht/MSX/d.cht", "c"*40),
            ("README.md", "d"*40),
        ]
        self.assertEqual(selected_delta(rows, {"a"*40, "c"*40}), [
            ("cht/MSX/b.cht", "b"*40), ("cht/MSX/c.cht", "b"*40)
        ])

    def test_tree_strict_mode_and_paths(self):
        raw = b"100644 blob " + (b"a"*40) + b"\tcht/MSX/a.cht\0"
        self.assertEqual(tree_rows(raw), [("cht/MSX/a.cht", "a"*40)])
        self.assertEqual(tree_rows(raw.replace(b"100644", b"100755")), [("cht/MSX/a.cht", "a"*40)])
        for bad in [
            b"120000 blob " + (b"a"*40) + b"\tcht/MSX/a.cht\0",
            b"100644 blob " + (b"a"*40) + b"\tcht/../bad.cht\0",
            b"100644 blob " + (b"a"*40) + b"\t/absolute.cht\0",
            raw+raw,
        ]:
            with self.assertRaises(ValueError):
                tree_rows(bad)


if __name__ == "__main__":
    unittest.main()
