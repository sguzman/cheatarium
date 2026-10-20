#!/usr/bin/env python3
"""Offline, synthetic fixtures for the Smash exact-build private source importer."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("fetch_smash_private", HERE / "fetch_smash_private.py")
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def mock_page(title=None, build=None, missing=None):
    title = title or MODULE.TITLE
    build = build or MODULE.BUILD
    rows = [f"<p>TID {title} BID {build}</p>", "<blockquote>"]
    for i, name in enumerate(MODULE.HEADING_NAMES):
        if name == missing:
            continue
        rows.extend([f"<div>[{name}]<br>", f"04000000 {i+1:08X} {i+100:08X}</div>"])
    rows.append("</blockquote>")
    return "\n".join(rows)


class PrivateSmashTests(unittest.TestCase):
    def test_exact_version_and_section_order(self):
        payload = MODULE.extract(mock_page())
        text = payload.decode()
        self.assertEqual(text.count("\n["), 24)
        self.assertIn("[1p invincible]\n04000000 00000001 00000064", text)
        self.assertIn("[boss no move]\n04000000 00000019 0000007C", text)
        self.assertFalse(MODULE.extract.__doc__ and "Eden" in payload.decode())

    def test_wrong_build_or_missing_code_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "expected Smash Title ID"):
            MODULE.extract(mock_page(build="BBBBBBBBBBBBBBBB"))
        with self.assertRaisesRegex(ValueError, "complete 25-section"):
            MODULE.extract(mock_page(missing="run+speed 1.5x"))
        with self.assertRaisesRegex(ValueError, "complete 25-section"):
            MODULE.extract(mock_page().replace("04000000 00000001", "INVALID 00000001"))

    def test_private_receipt_and_idempotence(self):
        body = MODULE.extract(mock_page())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "private"
            first = MODULE.store(body, "gbatemp-smash-1302-adaptation",
                                 MODULE.SOURCE["gbatemp-smash-1302-adaptation"], root)
            self.assertEqual(first["native_sections_staged"], 25)
            self.assertFalse(first["eden_modified"])
            self.assertEqual(first["status"], "staged_privately")
            file = root / MODULE.TITLE / (MODULE.BUILD + ".txt")
            sidecar = root / MODULE.TITLE / (MODULE.BUILD + ".source.json")
            self.assertEqual(file.read_bytes(), body)
            receipt = json.loads(sidecar.read_text())
            self.assertEqual(receipt["body_sha256"], hashlib.sha256(body).hexdigest())
            self.assertEqual(receipt["source_id"], "gbatemp-smash-1302-adaptation")
            self.assertFalse(receipt["gameplay_verified"])
            self.assertEqual(MODULE.store(body, receipt["source_id"],
                                         receipt["source_url"], root)["status"],
                             "already_staged_identical")
            with self.assertRaisesRegex(ValueError, "differs"):
                MODULE.store(body.replace(b"04000000", b"05000000", 1),
                             receipt["source_id"], receipt["source_url"], root)
            self.assertEqual(file.read_bytes(), body)

    def test_symlink_is_rejected(self):
        body = MODULE.extract(mock_page())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "private"
            root.symlink_to(Path(directory), target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "symlink"):
                MODULE.store(body, "panda-smash-1302",
                             MODULE.SOURCE["panda-smash-1302"], root)


if __name__ == "__main__":
    unittest.main()
