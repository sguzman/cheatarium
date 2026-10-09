#!/usr/bin/env python3
"""Synthetic regression tests for overlap-safe, source-exact SNES text patterns."""
import copy

from analyze_snes_patterns import analyze
from test_inspect_snes_joins import entry, record


def fixture():
    return {
        "schema_version": 1, "platform": "snes",
        "records": [
            record("fixture:a", "game", [
                entry(0, "AAAA+BBBB+0001"),
                entry(1, "AAAA+BBBB+0002"),
                entry(2, "AAAA+BBBB+0003"),
                entry(3, "AAAA+CCCC+0003"),
                entry(99, "AAAA+BBBB+0004", relation="revision-alternatives"),
            ]),
            record("fixture:b", "game", [entry(0, "AAAA+BBBB+0004")]),
            record("fixture:unrelated", "other", [
                entry(0, "AAAA+BBBB+0005"),
            ]),
        ],
    }


def test():
    bundle = fixture()
    report = analyze(bundle, "game", limit=1)
    assert report["unresolved_source_occurrences_examined"] == 5
    assert report["total_pattern_families"] == 2
    assert report["returned"] == 1 and report["has_more"]
    assert report["families_may_overlap"] and report["pattern_is_not_code_equivalence"]
    assert not report["safe_to_combine_or_auto_apply"]
    assert not report["execution_or_rom_compatibility_verified"]
    first = report["families"][0]
    assert first["source_record_id"] == "fixture:a"
    assert first["source_git_blob_sha"] == "a" * 40
    assert first["original_occurrences"] == 3
    assert first["distinct_varying_text_components"] == 3
    assert first["variable_component_position_zero_based"] == 2
    assert first["fixed_text_components"] == ["AAAA", "BBBB", None]
    assert [x["source_ordinal"] for x in first["sample_originals"]] == [0, 1, 2]
    assert first["sample_originals"][0]["original_code"] == "AAAA+BBBB+0001"

    second = analyze(bundle, "game", offset=1, limit=1)
    assert second["returned"] == 1 and not second["has_more"]
    assert second["families"][0]["original_occurrences"] == 2
    assert second["families"][0]["variable_component_position_zero_based"] == 1
    assert second["families"][0]["fixed_text_components"] == ["AAAA", None, "0003"]

    restricted = analyze(bundle, "game", source_record_id="fixture:b")
    assert restricted["total_pattern_families"] == 0
    assert restricted["unresolved_source_occurrences_examined"] == 1
    high_threshold = analyze(bundle, "game", min_members=4)
    assert high_threshold["total_pattern_families"] == 0

    reversed_bundle = copy.deepcopy(bundle)
    reversed_bundle["records"].reverse()
    for record_item in reversed_bundle["records"]:
        record_item["codes"].reverse()
    assert analyze(bundle, "game") == analyze(reversed_bundle, "game")

    publication = {
        "schema_version": 1, "platform": "snes",
        "claim_type": "externally-published-multi-part-source-code-text",
        "evidence_limit": "Historical listing, no execution evidence",
        "records": [{
            "candidate_game_key": "game", "source_record_id": "fixture:a",
            "source_git_blob_sha": "a" * 40, "source_ordinal": 1,
            "raw_code": "AAAA+BBBB+0002",
            "published_text_segments": ["AAAA", "BBBB", "0002"],
            "published_effect_description": "Example",
            "publication": {"url": "https://example.org/entry",
                            "reference": "entry 1", "source_revision": "undated"},
            "execution_observed": False, "rom_match_verified": False,
            "safe_to_auto_apply": False,
        }],
    }
    witnessed = analyze(bundle, "game", publications=publication)
    assert witnessed["historical_publication_witnesses"] == 1
    assert witnessed["families"][0]["publication_witnessed_occurrences"] == 1

    for args in ({"min_members": 1}, {"offset": -1}, {"limit": 0},
                 {"limit": 101}):
        try:
            analyze(bundle, "game", **args)
        except ValueError:
            pass
        else:
            raise AssertionError(f"Bad pattern analyzer argument accepted: {args}")
    print("OK: SNES text families preserve provenance, overlap and uncertainty")


if __name__ == "__main__":
    test()
