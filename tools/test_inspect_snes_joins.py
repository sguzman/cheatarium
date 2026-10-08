#!/usr/bin/env python3
"""Adversarial, ROM-free tests for read-only SNES join dossier extraction."""
import copy
import json

from inspect_snes_joins import dossier


def entry(ordinal, raw, relation="unresolved"):
    return {
        "role": "code", "ordinal": ordinal, "code": raw,
        "description": f"Original description {ordinal}",
        "source_enabled": True, "verification": "imported-unverified",
        "composition": {
            "relation": relation, "alternatives": [], "evidence": [],
            "rom_match_verified": False, "simultaneous_execution_confirmed": False,
        },
    }


def record(record_id, game, codes):
    return {
        "id": record_id, "candidate_game_key": game, "title_hint": "Original title",
        "region_hint": "USA", "format_hint": "game-genie",
        "provenance": {
            "source_id": "example-archive", "revision": "pinned-sha",
            "git_blob_sha": "a" * 40, "archive_path": f"archive/{record_id}.cht",
        },
        "codes": codes,
    }


def fixture():
    return {
        "schema_version": 1, "platform": "snes",
        "records": [
            record("source-b", "example", [
                entry(9, "AAAA+BBBB"), entry(10, "AAAA+BBBB"),
                entry(11, "CCCC+DDDD", "revision-alternatives"),
            ]),
            record("source-a", "example", [entry(4, " AAAA + CCCC + DDDD ")]),
            record("source-c", "other-game", [entry(1, "ZZZZ+XXXX")]),
        ],
    }


def rejects(fn):
    try:
        fn()
    except ValueError:
        return
    raise AssertionError("Invalid dossier input was accepted")


def test():
    bundle = fixture()
    report = dossier(bundle, "example", limit=2)
    assert report["total_unresolved_source_occurrences"] == 3
    assert report["distinct_original_code_strings"] == 2
    assert report["distinct_source_records"] == 2
    assert report["returned"] == 2 and report["has_more"]
    assert report["source_record_counts"] == [
        {"source_record_id": "source-b", "count": 2},
        {"source_record_id": "source-a", "count": 1},
    ]
    assert report["text_segment_count_distribution"] == [
        {"text_segment_count": 2, "count": 2},
        {"text_segment_count": 3, "count": 1},
    ]
    assert report["occurrences"][0]["source_record_id"] == "source-a"
    assert report["occurrences"][0]["original_code"] == " AAAA + CCCC + DDDD "
    assert report["occurrences"][0]["source_ordinal"] == 4
    assert report["occurrences"][0]["original_description"] == "Original description 4"
    assert report["occurrences"][0]["provenance"]["git_blob_sha"] == "a" * 40
    assert report["occurrences"][0]["original_source_enabled"] is True
    assert report["source_enabled_is_not_activation"] is True
    assert not report["effect_or_compatibility_verified"]
    assert all(x["relation"] == "unresolved" for x in
               (bundle["records"][0]["codes"][0]["composition"],))

    second = dossier(bundle, "example", limit=2, offset=2)
    assert [x["source_ordinal"] for x in second["occurrences"]] == [10]
    assert not second["has_more"]
    assert dossier(bundle, "example", offset=300)["returned"] == 0
    source = dossier(bundle, "example", source_record_id="source-b")
    assert source["total_unresolved_source_occurrences"] == 2
    assert source["distinct_source_records"] == 1
    assert all(x["source_record_id"] == "source-b" for x in source["occurrences"])
    assert dossier(bundle, "missing")["total_unresolved_source_occurrences"] == 0

    reverse = copy.deepcopy(bundle)
    reverse["records"].reverse()
    assert json.dumps(dossier(bundle, "example")) == json.dumps(dossier(reverse, "example"))

    rejects(lambda: dossier({"schema_version": 2, "platform": "snes"}, "example"))
    rejects(lambda: dossier(bundle, ""))
    rejects(lambda: dossier(bundle, "example", offset=-1))
    rejects(lambda: dossier(bundle, "example", limit=0))
    rejects(lambda: dossier(bundle, "example", limit=501))
    altered = copy.deepcopy(bundle)
    altered["records"][0]["codes"][0]["composition"]["relation"] = "simultaneous"
    rejects(lambda: dossier(altered, "example"))
    altered = copy.deepcopy(bundle)
    altered["records"][0]["codes"][0]["composition"]["evidence"] = [{"url": "guess"}]
    rejects(lambda: dossier(altered, "example"))
    altered = copy.deepcopy(bundle)
    altered["records"][0]["codes"][0]["composition"]["rom_match_verified"] = True
    rejects(lambda: dossier(altered, "example"))
    publication = {
        "schema_version": 1, "platform": "snes",
        "claim_type": "externally-published-multi-part-source-code-text",
        "evidence_limit": "Publication is not verified execution",
        "records": [{
            "candidate_game_key": "example", "source_record_id": "source-b",
            "source_git_blob_sha": "f" * 40, "source_ordinal": 9,
            "raw_code": "AAAA+BBBB",
            "published_text_segments": ["AAAA", "BBBB"],
            "published_effect_description": "Original published effect",
            "publication": {"url": "https://example.org/faq",
                            "reference": "Example 9", "source_revision": "undated"},
            "execution_observed": False, "rom_match_verified": False,
            "safe_to_auto_apply": False,
        }],
    }
    witnessed = dossier(bundle, "example", publication_registry=publication)
    assert witnessed["historical_publication_witnesses"] == 1
    match = next(x for x in witnessed["occurrences"] if x["source_ordinal"] == 9)
    assert match["historical_publication_witness"]["publication"]["reference"] == "Example 9"
    assert not match["historical_publication_witness"]["execution_observed"]
    assert not match["historical_publication_witness"]["safe_to_auto_apply"]
    missing = dossier(bundle, "other-game", publication_registry=publication)
    assert missing["historical_publication_witnesses"] == 0
    altered_publication = copy.deepcopy(publication)
    altered_publication["records"][0]["raw_code"] = "AAA+WRONG"
    rejects(lambda: dossier(bundle, "example", publication_registry=altered_publication))

    print("OK: complete, paginated, source-exact SNES dossiers with no invented code semantics")


if __name__ == "__main__":
    test()
