#!/usr/bin/env python3
"""Offline tests for immutable upstream rights evidence checks."""
import unittest

from verify_rights_pins import github_tree, verify


def report():
    return {
        "format": "cheatarium-acquisition-rights-review-v1",
        "metadata_only": True, "no_automatic_rights_clearance": True,
        "cheat_or_patch_bodies_included": False,
        "source_reviews_with_pinned_evidence": 1,
        "reviewed_sources": [{
            "source_id": "demo", "repository": "https://github.com/demo/cheats",
            "pinned_git_tree": "a" * 40,
            "root_license_paths_observed": ["LICENSE"],
            "redistribution_cleared": False,
            "documentary_evidence": [
                {"upstream_path": "README.md", "git_blob_sha": "b" * 40},
                {"upstream_path": "LICENSE", "git_blob_sha": "c" * 40},
            ],
        }],
    }


def tree():
    return {
        "sha": "a" * 40, "truncated": False,
        "tree": [
            {"path": "README.md", "type": "blob", "sha": "b" * 40},
            {"path": "LICENSE", "type": "blob", "sha": "c" * 40},
            {"path": "cheats", "type": "tree", "sha": "d" * 40},
        ],
    }


class PinTests(unittest.TestCase):
    def test_root_metadata_only_validation(self):
        requests = []
        def fake(repo, sha):
            requests.append((repo, sha))
            return tree()
        result = verify(report(), fake)
        self.assertEqual(requests, [("https://github.com/demo/cheats", "a" * 40)])
        self.assertEqual(result["documentary_git_blobs_checked"], 2)
        self.assertEqual(result["source_revisions_checked"], 1)
        self.assertEqual(result["cheat_code_payloads_downloaded"], 0)
        self.assertEqual(result["original_author_rights_cleared"], 0)

    def test_bad_document_hash_and_root_license_coverage_rejected(self):
        for bad in ("doc", "license", "tree", "truncated"):
            item = tree()
            if bad == "doc":
                item["tree"][0]["sha"] = "e" * 40
            elif bad == "license":
                item["tree"].append({"path": "COPYING", "type": "blob",
                                     "sha": "f" * 40})
            elif bad == "tree":
                item["sha"] = "f" * 40
            elif bad == "truncated":
                item["truncated"] = True
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                verify(report(), lambda _r, _s: item)

    def test_cannot_assert_clearance(self):
        d = report()
        d["reviewed_sources"][0]["redistribution_cleared"] = True
        with self.assertRaises(ValueError):
            verify(d, lambda _r, _s: tree())

    def test_unsafe_url_rejected_without_network(self):
        for repo in ("http://github.com/a/b", "https://elsewhere.com/a/b",
                     "https://github.com/a/b?x=1", "https://github.com/a",
                     "https://github.com/a/b/extra"):
            with self.subTest(repo=repo), self.assertRaises(ValueError):
                github_tree(repo, "a" * 40)


if __name__ == "__main__":
    unittest.main()
