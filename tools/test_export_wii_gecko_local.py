#!/usr/bin/env python3
"""Pure synthetic data only; never include actual contributor code in CI files."""
import json
from pathlib import Path
import tempfile
import unittest

from export_wii_gecko_local import (
    FORMAT, build_document, safe_output_path, verify_sidecar, write_sidecar,
)


def fabricated():
    sources = []
    for i in range(29):
        groups = []
        n = 2 if i < 16 else 1  # 16*2 + 13 = 45 synthetic groups
        for j in range(n):
            group_num = 1 + j
            pair_count = 1
            groups.append({
                "group_index": group_num,
                "title": "FAKE SELF-CONTAINED SAMPLE",
                "notes": ["Synthetic only"],
                "code_lines": ["01234567 89ABCDEF"] * (pair_count if pair_count else 1),
                "instruction_pair_count": pair_count if pair_count else 1,
                "source_enabled": None,
            })
        sources.append({
            "claimed_disc_id": f"X{i:05d}",
            "source_revision_token": "v0" if i == 0 else None,
            "source_variant_token": "base",
            "game_family_title_hint": "Synthetic Game",
            "source_compatibility_warning": "Unknown build",
            "source_id": "synthetic-" + str(i),
            "local_path": f"private/synthetic{i}.txt",
            "original_git_blob_sha1": "a"*40,
            "upstream_original_url": "https://github.com/a/b/blob/" + "b"*40 + "/source.txt",
            "groups": groups,
        })
    # The real sidecar expects 5611 total native instruction pairs; use a
    # large dummy repeated instruction string solely to hit that invariant.
    total = sum(len(g["code_lines"]) for s in sources for g in s["groups"])
    diff = 5611 - total
    first = sources[0]["groups"][0]
    first["code_lines"] += ["01234567 89ABCDEF"] * diff
    first["instruction_pair_count"] = len(first["code_lines"])
    return sources


class WiiSidecarTests(unittest.TestCase):
    def test_local_sidecar_keeps_versions_and_is_disabled(self):
        doc = build_document(fabricated())
        self.assertEqual(doc["format"], FORMAT)
        self.assertEqual(len(doc["records"]), 45)
        self.assertEqual(doc["native_instruction_pair_count"], 5611)
        self.assertTrue(doc["all_groups_disabled_by_default"])
        self.assertTrue(all(not x["enabled_for_execution"] for x in doc["records"]))
        self.assertTrue(all(not x["game_build_authenticated"] for x in doc["records"]))
        self.assertIn("native_code_lines", doc["records"][0])

    def test_private_written_bytes_verify_and_corruption_is_detected(self):
        with tempfile.TemporaryDirectory() as temp:
            tempdir = Path(temp)
            repo = tempdir / "publicrepo"
            repo.mkdir()
            dest = tempdir / "private" / "codes.json"
            payload = json.dumps(build_document(fabricated())).encode("utf-8")
            write_sidecar(dest, payload, root=repo)
            self.assertEqual(dest.read_bytes(), payload)
            verify_sidecar(dest, payload, root=repo)
            with self.assertRaises(FileExistsError):
                write_sidecar(dest, payload, root=repo)
            write_sidecar(dest, payload, replace=True, root=repo)
            dest.write_bytes(b"modified")
            with self.assertRaises(ValueError):
                verify_sidecar(dest, payload, root=repo)

    def test_public_git_destination_and_symlink_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "publicrepo"
            root.mkdir()
            with self.assertRaises(ValueError):
                safe_output_path(root / "generated/v1/cheats.json", root=root)
            other = Path(temp) / "private.json"
            other.symlink_to(Path(temp) / "nowhere.json")
            with self.assertRaises(ValueError):
                write_sidecar(other, b"code", replace=True, root=root)

    def test_no_silent_revision_guess_from_source(self):
        docs = fabricated()
        docs[1]["claimed_disc_id"] = docs[0]["claimed_disc_id"]
        docs[1]["source_revision_token"] = "v1"
        data = build_document(docs)
        variants = [r for r in data["records"] if r["claimed_disc_id"] == "X00000"]
        self.assertEqual({r["source_revision_token"] for r in variants}, {"v0", "v1"})
        self.assertFalse(any(r["enabled_for_execution"] for r in variants))


if __name__ == "__main__":
    unittest.main()
