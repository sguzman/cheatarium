#!/usr/bin/env python3
"""Offline regression tests for pinned external rights evidence."""
import unittest

from audit_acquisition_rights import compile_review


def priority(ids):
    return {
        "format": "cheatarium-acquisition-priorities-v1",
        "source_rights_cleared_for_import": False,
        "sources": [
            {"source_id": sid, "exclusive_unarchived_cheat_labeled_blobs": n,
             "distinct_unarchived_cheat_labeled_blobs": n + 2}
            for n, sid in enumerate(ids, start=1)
        ],
    }


def index(sid):
    return {"id": sid, "platform": "ps2", "repository": "https://github.com/demo/" + sid,
            "snapshot_commit": "a" * 40, "git_tree_sha": "b" * 40,
            "rights_status": "link_only_rights_review"}


def entry(sid):
    return {
        "source_id": sid, "review_state": "rights_unresolved_link_only",
        "blocker": "mixed_unlicensed_provenance", "root_license_paths_observed": [],
        "root_license_scope_assessment": "no_root_grant_found",
        "recommended_clearance": "Check original contributor permission",
        "evidence": [{"upstream_path": "README.md", "git_blob_sha": "c" * 40,
                      "evidence_type": "mixed_source_attribution"}],
    }


def dossier(entries):
    return {
        "schema_version": 1, "format": "cheatarium-pinned-source-rights-evidence-v1",
        "review_state": "all_reviewed_sources_remain_link_only_rights_review",
        "source_reviews": entries,
    }


class RightsReviewTests(unittest.TestCase):
    def test_rank_and_unreviewed_sources_and_pin_links(self):
        result = compile_review(dossier([entry("b")]),
                                priority(["a", "b"]), [index("a"), index("b")])
        self.assertEqual(result["source_catalogs"], 2)
        self.assertEqual(result["source_reviews_with_pinned_evidence"], 1)
        self.assertEqual(result["source_catalogs_not_yet_documented"], 1)
        self.assertEqual(result["unreviewed_source_ids_in_priority_order"], ["a"])
        reviewed = result["reviewed_sources"][0]
        self.assertEqual(reviewed["priority_rank"], 2)
        self.assertEqual(reviewed["exclusive_unarchived_cheat_labeled_blobs"], 2)
        self.assertTrue(reviewed["documentary_evidence"][0]["pinned_url"].endswith(
            "/blob/" + "a" * 40 + "/README.md"))
        self.assertFalse(reviewed["redistribution_cleared"])
        self.assertFalse(result["rights_reviews_are_legal_permissions"])

    def test_license_scopes_require_documentary_evidence(self):
        r = entry("a")
        r["root_license_paths_observed"] = ["LICENSE"]
        r["root_license_scope_assessment"] = "processing_scripts_mit_only"
        with self.assertRaises(ValueError):
            compile_review(dossier([r]), priority(["a"]), [index("a")])
        r["evidence"].append({
            "upstream_path": "LICENSE", "git_blob_sha": "d" * 40,
            "evidence_type": "explicit_cheat_author_copyright_reservation",
        })
        report = compile_review(dossier([r]), priority(["a"]), [index("a")])
        self.assertEqual(report["reviewed_sources"][0]["root_license_paths_observed"],
                         ["LICENSE"])

    def test_rejects_duplicate_or_invented_evidence(self):
        bad = [
            [entry("a"), entry("a")],
            [{**entry("a"), "blocker": "licensed_everything"}],
            [{**entry("a"), "recommended_clearance": ""}],
            [{**entry("a"), "review_state": "approved"}],
            [{**entry("a"), "evidence": [
                {"upstream_path": "../README.md", "git_blob_sha": "c" * 40,
                 "evidence_type": "mixed_source_attribution"}]}],
            [{**entry("a"), "evidence": [
                {"upstream_path": "README.md", "git_blob_sha": "invalid",
                 "evidence_type": "mixed_source_attribution"}]}],
            [{**entry("a"), "root_license_paths_observed": ["LICENSE"]}],
        ]
        for entries in bad:
            with self.subTest(entries=entries), self.assertRaises(ValueError):
                compile_review(dossier(entries), priority(["a"]), [index("a")])

    def test_no_evidence_is_not_clearance(self):
        with self.assertRaises(ValueError):
            compile_review(dossier([]), priority(["a"]), [index("a")])
        with self.assertRaises(ValueError):
            compile_review(dossier([entry("a")]),
                           priority(["a"]), [{**index("a"), "rights_status": "cleared"}])


if __name__ == "__main__":
    unittest.main()
