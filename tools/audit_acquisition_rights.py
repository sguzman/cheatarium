#!/usr/bin/env python3
"""Attach pinned upstream rights evidence to the bulk acquisition priority queue.

Input evidence consists exclusively of source metadata and Git blob hashes of
upstream README/LICENSE/credit docs. No cheat bodies are copied or interpreted.
This is a reproducible triage register, not legal clearance or legal advice.
"""
import argparse
import json
import re
import urllib.parse
from pathlib import Path

from build_acquisition_priorities import build as build_priorities
from external_bulk import ROOT, load

EVIDENCE = Path("sources/rights-evidence-v1.json")
OUTPUT = Path("generated/v1/reports/acquisition-rights-review.json")
FORMAT = "cheatarium-acquisition-rights-review-v1"
INPUT_FORMAT = "cheatarium-pinned-source-rights-evidence-v1"
STATUS = "rights_unresolved_link_only"
SHA = re.compile(r"[a-f0-9]{40}\Z")
BLOCKERS = {
    "mixed_unlicensed_provenance",
    "explicit_original_author_rights_reservation",
    "third_party_mirror_without_data_grant",
    "aggregator_license_with_third_party_contributions",
    "multi_author_database_without_data_grant",
    "community_contributions_without_root_grant",
}
SCOPES = {
    "no_root_grant_found", "processing_scripts_mit_only",
    "aggregator_mit_no_original_author_clearance",
}
EVIDENCE_TYPES = {
    "mixed_source_attribution", "third_party_fork_integration",
    "explicit_cheat_author_copyright_reservation",
    "user_authored_cheat_catalog", "multiple_upstream_cheat_sources",
    "explicit_external_mirror_credits", "root_mit_notice",
    "multiple_mod_creator_credits", "community_submissions",
    "individual_code_author_credits", "community_code_contributions",
}


def safe_doc_path(path):
    return (isinstance(path, str) and path != ""
            and not path.startswith("/")
            and "\\" not in path and "\x00" not in path
            and all(part not in ("", ".", "..") for part in path.split("/"))
            and path.rsplit("/", 1)[-1].lower().endswith((
                ".md", "license", ".txt")))


def compile_review(evidence_doc, priorities, indexes):
    if (evidence_doc.get("schema_version") != 1
            or evidence_doc.get("format") != INPUT_FORMAT
            or evidence_doc.get("review_state") !=
               "all_reviewed_sources_remain_link_only_rights_review"
            or priorities.get("format") != "cheatarium-acquisition-priorities-v1"
            or priorities.get("source_rights_cleared_for_import") is not False):
        raise ValueError("Rights evidence or acquisition priority schema changed")
    by_id = {x["id"]: x for x in indexes}
    if len(by_id) != len(indexes):
        raise ValueError("Duplicate external sources")
    priority_by_id = {x["source_id"]: (i + 1, x)
                      for i, x in enumerate(priorities["sources"])}
    if set(by_id) != set(priority_by_id):
        raise ValueError("Rights and acquisition catalog source IDs disagree")
    reviewed = []
    seen = set()
    for r in evidence_doc["source_reviews"]:
        sid = r.get("source_id")
        if sid in seen or sid not in by_id:
            raise ValueError("Duplicate or unknown reviewed source")
        seen.add(sid)
        src = by_id[sid]
        if (src.get("rights_status") != "link_only_rights_review"
                or r.get("review_state") != STATUS
                or r.get("blocker") not in BLOCKERS
                or r.get("root_license_scope_assessment") not in SCOPES
                or not isinstance(r.get("recommended_clearance"), str)
                or not r["recommended_clearance"].strip()
                or not SHA.fullmatch(src.get("snapshot_commit", ""))
                or not SHA.fullmatch(src.get("git_tree_sha", ""))):
            raise ValueError("Source license state or pinned revision invalid")
        root_licenses = r.get("root_license_paths_observed")
        if (not isinstance(root_licenses, list)
                or len(set(root_licenses)) != len(root_licenses)
                or any(not safe_doc_path(x) or "/" in x
                       or not re.fullmatch(r"(?:LICENSE|COPYING|NOTICE)(?:\..*)?",
                                           x, flags=re.I) for x in root_licenses)):
            raise ValueError("Malformed observed root license inventory")
        scope = r["root_license_scope_assessment"]
        if (bool(root_licenses) != (scope != "no_root_grant_found")):
            raise ValueError("Root license observation contradicts assessment")
        evidences = r.get("evidence")
        if not isinstance(evidences, list) or not evidences:
            raise ValueError("Reviewed source must have documentary evidence")
        paths = set()
        emitted = []
        for record in evidences:
            p, sha, kind = (record.get("upstream_path"), record.get("git_blob_sha"),
                            record.get("evidence_type"))
            if (not safe_doc_path(p) or p in paths
                    or not isinstance(sha, str) or not SHA.fullmatch(sha)
                    or kind not in EVIDENCE_TYPES):
                raise ValueError("Malformed or duplicate documentary evidence")
            paths.add(p)
            emitted.append({
                "upstream_path": p, "git_blob_sha": sha,
                "evidence_type": kind,
                "pinned_url": (src["repository"] + "/blob/" +
                               src["snapshot_commit"] + "/" +
                               urllib.parse.quote(p, safe="/")),
            })
        if not set(root_licenses).issubset(paths):
            raise ValueError("Root license notice must have pinned blob evidence")
        priority, measured = priority_by_id[sid]
        reviewed.append({
            "source_id": sid, "platform": src["platform"],
            "repository": src["repository"],
            "pinned_revision": src["snapshot_commit"],
            "pinned_git_tree": src["git_tree_sha"],
            "priority_rank": priority,
            "exclusive_unarchived_cheat_labeled_blobs":
                measured["exclusive_unarchived_cheat_labeled_blobs"],
            "distinct_unarchived_cheat_labeled_blobs":
                measured["distinct_unarchived_cheat_labeled_blobs"],
            "rights_status": "link_only_rights_review",
            "redistribution_cleared": False,
            "review_state": STATUS,
            "blocker": r["blocker"],
            "root_license_paths_observed": root_licenses,
            "root_license_scope_assessment": scope,
            "documentary_evidence": emitted,
            "next_clearance_action": r["recommended_clearance"],
        })
    if not seen:
        raise ValueError("No documentary rights reviews")
    reviewed.sort(key=lambda row: (row["priority_rank"], row["source_id"]))
    by_rank = sorted(priority_by_id.items(), key=lambda x: x[1][0])
    unreviewed = [sid for sid, _ in by_rank if sid not in seen]
    return {
        "schema_version": 1, "format": FORMAT, "metadata_only": True,
        "cheat_or_patch_bodies_included": False,
        "rights_reviews_are_legal_permissions": False,
        "license_absence_proves_copyright_or_prohibition": False,
        "no_automatic_rights_clearance": True,
        "source_catalogs": len(indexes), "source_reviews_with_pinned_evidence": len(reviewed),
        "source_catalogs_not_yet_documented": len(unreviewed),
        "reviewed_exclusive_unarchived_cheat_labeled_blobs":
            sum(x["exclusive_unarchived_cheat_labeled_blobs"] for x in reviewed),
        "priority_ranking_does_not_assert_working_cheats": True,
        "reviewed_sources": reviewed,
        "unreviewed_source_ids_in_priority_order": unreviewed,
        "limitations": [
            "README, LICENSE and contributor files are repository claims, not verified original code author grants.",
            "Absence of a root license file is not proof that copyright exists or that no permission can be obtained.",
            "Even an MIT root license may cover only the aggregator's own work.",
            "Root tree/document hashes are pinned research observations; this offline tool does not re-fetch GitHub or certify contributor provenance.",
            "No file is imported into archived code bundles because of this report.",
        ],
    }


def build(root=ROOT):
    evidence = json.loads((root / EVIDENCE).read_text("utf-8"))
    return compile_review(evidence, build_priorities(root), load(root))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    flags = parser.add_mutually_exclusive_group(required=True)
    flags.add_argument("--write", action="store_true")
    flags.add_argument("--check", action="store_true")
    args = parser.parse_args()
    report = build()
    payload = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    out = ROOT / OUTPUT
    if args.write:
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(payload, "utf-8")
    elif not out.exists() or out.read_text("utf-8") != payload:
        raise SystemExit("Pinned acquisition rights review report missing or stale")
    print("OK: pinned rights evidence for",
          report["source_reviews_with_pinned_evidence"],
          "link-only source catalogs; zero blanket code-body grants")


if __name__ == "__main__":
    main()
