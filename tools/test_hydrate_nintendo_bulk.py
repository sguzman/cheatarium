#!/usr/bin/env python3
"""Network-free checksum/path/atomically-staged Nintendo source cache tests."""
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

from hydrate_nintendo_bulk import (git_blob_sha, validate_blob, safe_components,
                                  add_file, build_into, check, hydrate, FORMAT)


def pack(member, data):
    out = io.BytesIO()
    with zipfile.ZipFile(out, "w") as z:
        z.writestr(member, data)
    return out.getvalue()


class TestNintendoHydrate(unittest.TestCase):
    def test_reject_wrong_original_blob_or_size(self):
        raw = b"cheat lines"
        correct = ["code.txt", git_blob_sha(raw), len(raw)]
        validate_blob(correct, raw)
        with self.assertRaises(ValueError):
            validate_blob(correct, raw + b"x")
        with self.assertRaises(ValueError):
            validate_blob(["code.txt", "0"*40, len(raw)], raw)

    def test_strict_original_paths(self):
        for name in ("", "/etc/passwd", "../file.txt", "a/../b", "a\\b", "a//b"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                safe_components(name)

    def test_add_file_blocks_collisions_and_preserves_exact_bytes(self):
        with tempfile.TemporaryDirectory() as root:
            out = Path(root)
            records = {}
            add_file(out, records, "gamecube/NTSC-J/ABCD01.txt", b"code", "gc", "ABCD01", "a"*40)
            self.assertEqual((out / "gamecube/NTSC-J/ABCD01.txt").read_bytes(), b"code")
            with self.assertRaises(ValueError):
                add_file(out, records, "gamecube/NTSC-J/ABCD01.txt", b"other", "gc", "ABCD01", "a"*40)

    def test_real_pinned_manifest_has_required_source_and_member_scope(self):
        from hydrate_nintendo_bulk import pinned_sources
        sources, members = pinned_sources()
        self.assertEqual([len(s["files"]) for s in sources], [3, 9, 4])
        self.assertEqual(len(members["records"]), 375)
        self.assertEqual(members["file_occurrences_by_region"], {"NTSC-J": 8, "NTSC-U": 138, "PAL": 229})

    def test_check_rejects_missing_and_corrupt_originals(self):
        with tempfile.TemporaryDirectory() as path:
            root = Path(path)
            with self.assertRaises(FileNotFoundError):
                check(root)
            docs = {"schema_version": 1, "format": FORMAT,
                    "private_local_only": True, "original_author_redistribution_rights_cleared": False,
                    "source_ids": ["leumas-gamecube-wiird-zips", "katsumaki-wiiu-xml", "ucodes-wiiu-json"],
                    "gamecube_files": 375, "wiiu_files": 13, "records": []}
            (root / "manifest.json").write_text(json.dumps(docs))
            with self.assertRaises(ValueError):
                check(root)

    def test_public_repository_destination_is_refused_before_network(self):
        from hydrate_nintendo_bulk import ROOT
        with self.assertRaises(ValueError):
            hydrate(ROOT / "archive/unlicensed-nintendo", fetch=lambda *_: self.fail("network"))

    def test_fixture_gamecube_original_mismatch_rejected(self):
        from hydrate_nintendo_bulk import build_into
        source = {"id": "leumas-gamecube-wiird-zips"}
        a = pack("ABCD01.txt", b"cheat")
        sources = [
            {"id": "leumas-gamecube-wiird-zips",
             "files": [["NTSC-J codes.zip", git_blob_sha(a), len(a)]],
             "repository": "https://github.com/example/cheats", "snapshot_commit": "a"*40},
            {"id": "katsumaki-wiiu-xml", "files": []},
            {"id": "ucodes-wiiu-json", "files": []},
        ]
        members = {"records": [
            {"region":"NTSC-J", "original_container_path":"NTSC-J codes.zip",
             "native_zip_member_path":"ABCD01.txt", "source_member_uncompressed_bytes": 99,
             "original_container_git_blob_sha1":git_blob_sha(a)},
        ]}
        with tempfile.TemporaryDirectory() as tmp, self.assertRaises(ValueError):
            build_into(Path(tmp), sources, members, lambda *_: a)


if __name__ == "__main__":
    unittest.main()
