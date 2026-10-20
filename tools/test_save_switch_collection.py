#!/usr/bin/env python3
"""Offline regression of complete six-title source preservation in Cheatarium."""
from __future__ import annotations

import contextlib
import hashlib
import io
import json
from pathlib import Path
import sys
from tempfile import TemporaryDirectory
from unittest import TestCase, main, mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import save_switch_collection as saver

REAL_ROOT = Path(__file__).resolve().parents[1]


class SaveSwitchTests(TestCase):
    def test_native_parser_and_exact_fingerprint(self):
        self.assertEqual(saver.native_sections(
            b"[Health]\n04000000 00112233 00000001\n\n[Money]\n04000000 00112233 00000002\n"
        ), [
            ("[Health]", "04000000 00112233 00000001"),
            ("[Money]", "04000000 00112233 00000002")
        ])
        with self.assertRaises(ValueError):
            saver.native_sections(b"[Unsafe]\nfrom urllib.request import urlopen\n")
        with self.assertRaises(ValueError):
            saver.native_sections(b"[Empty]\n[Other]\n04000000 00112233 00000003\n")
        self.assertEqual(saver.digest("04000000 00112233 00000001"),
                         hashlib.sha256(b"04000000 00112233 00000001").hexdigest())

    def test_all_174_save_and_verify_without_network_or_eden(self):
        with TemporaryDirectory() as td:
            root = Path(td)
            (root / "research").mkdir()
            (root / "sources").mkdir()
            (root / "archive").mkdir()
            (root / "curated").mkdir()
            report = json.loads((REAL_ROOT / "research/switch-exact-build-coverage-v1.json").read_text("utf-8"))
            original = json.loads((REAL_ROOT / "research/switch-personal-curation-v1.json").read_text("utf-8"))
            synthetic: dict[str, bytes] = {}
            for game in report["games"]:
                lines = []
                for row in game["sections"]:
                    # Unique deterministic hex words for this synthetic test only.
                    hex_line = f"04000000 00000000 {row['ordinal']:08X}"
                    row["source_code_sha256"] = saver.digest(hex_line)
                    row["state"] = "linked_only_no_public_code_body"
                    row["archive_path"] = None
                    row["curated_cheat_id"] = None
                    lines.extend((row["heading"], hex_line, ""))
                synthetic[game["title_id"]] = "\n".join(lines).encode()
                game["archived_exact_section_count"] = 0
                game["missing_body_count"] = len(game["sections"])
            report["counts"]["archived_exact_sections"] = 0
            report["counts"]["linked_only_sections"] = 174
            (root / "research/switch-exact-build-coverage-v1.json").write_text(
                json.dumps(report, indent=2) + "\n", encoding="utf-8")
            (root / "research/switch-personal-curation-v1.json").write_text(
                json.dumps(original, indent=2) + "\n", encoding="utf-8")
            def fake_source(game, manifest, cache):
                return synthetic[game["title_id"]], "https://example.com/fixture", "fixture"
            with (mock.patch.object(saver, "ROOT", root),
                  mock.patch.object(saver, "source_bytes", side_effect=fake_source),
                  mock.patch.object(sys, "argv", ["save_switch_collection.py", "--save"])):
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(saver.main(), 0)
                coverage = json.loads((root / "research/switch-exact-build-coverage-v1.json").read_text("utf-8"))
                self.assertEqual(coverage["counts"]["archived_exact_sections"], 174)
                self.assertEqual(coverage["counts"]["linked_only_sections"], 0)
                self.assertEqual(len(list((root / "archive").glob("*/*/*.txt"))), 6)
                self.assertEqual(len(list((root / "curated/switch").glob("*/cheats.json"))), 6)
                with contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(saver.main(), 0)  # idempotent --save
                with mock.patch.object(sys, "argv", ["save_switch_collection.py", "--verify"]):
                    with contextlib.redirect_stdout(io.StringIO()):
                        self.assertEqual(saver.main(), 0)
                # One corrupted archived original must never be overwritten.
                saved = root / "archive/switch-exact-hamlet/0100000000010000/B424BE150A8E7D78.txt"
                saved.write_bytes(b"tampered")
                with self.assertRaisesRegex(ValueError, "Preserved source integrity mismatch"):
                    saver.save_files(root, saver.prepare(root, root)[0])


if __name__ == "__main__":
    main()
