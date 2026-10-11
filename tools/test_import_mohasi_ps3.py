#!/usr/bin/env python3
"""Fast ROM-free regression checks for the pinned PS3 bulk importer."""
import unittest
from import_mohasi_ps3 import (
    EXPECTED_COMPILED, EXPECTED_SOURCE, git_blob, selected_paths,
)


class TestMohasiPS3Import(unittest.TestCase):
    def test_git_blob_identity(self):
        self.assertEqual(
            git_blob(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a",
        )

    def test_complete_pinned_inventory(self):
        paths = ["LICENSE", "README.md"]
        paths += [f"compiled/BCES{i:05d}.txt" for i in range(EXPECTED_COMPILED)]
        paths += [f"source/BCES{i:05d}.txt" for i in range(EXPECTED_SOURCE)]
        paths += ["tools/compile.py", "votes/.gitkeep"]
        chosen, counts = selected_paths(paths)
        self.assertEqual(len(chosen), EXPECTED_COMPILED + EXPECTED_SOURCE + 2)
        self.assertEqual(counts, {"compiled": 4343, "source": 13343})

    def test_rejects_traversal_duplicate_and_drift(self):
        paths = ["source/../secret.txt"]
        with self.assertRaises(ValueError):
            selected_paths(paths)
        with self.assertRaises(ValueError):
            selected_paths(["compiled/BCES11111.txt"] * 2)
        with self.assertRaises(RuntimeError):
            selected_paths(["LICENSE", "README.md", "source/BCES11111.txt"])


if __name__ == "__main__":
    unittest.main()
