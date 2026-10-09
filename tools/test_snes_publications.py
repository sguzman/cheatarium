#!/usr/bin/env python3
"""ROM-free adversarial tests for externally witnessed SNES group listings."""
import copy

from validate_snes_publications import CLAIM_TYPE, validate_registry


def fixture():
    source_id = "fixture:snes/game.cht"
    code = "ABCD-1234+EFGH-5678"
    bundle = {"schema_version": 1, "platform": "snes", "records": [{
        "id": source_id, "candidate_game_key": "game", "title_hint": "Game",
        "provenance": {"git_blob_sha": "a" * 40},
        "codes": [{"ordinal": 7, "role": "code", "code": code,
                   "composition": {"relation": "unresolved"}}],
    }]}
    registry = {
        "schema_version": 1, "platform": "snes", "claim_type": CLAIM_TYPE,
        "evidence_limit": "A published listing, not proof of execution.",
        "records": [{
            "candidate_game_key": "game",
            "source_record_id": source_id,
            "source_git_blob_sha": "a" * 40,
            "source_ordinal": 7,
            "raw_code": code,
            "published_text_segments": ["ABCD-1234", "EFGH-5678"],
            "published_effect_description": "As stated in external document",
            "publication": {"url": "https://example.org/manual",
                            "reference": "Entry 8", "source_revision": "undated"},
            "execution_observed": False,
            "rom_match_verified": False,
            "safe_to_auto_apply": False,
        }],
    }
    return registry, bundle


def fails(registry, bundle):
    try:
        validate_registry(registry, bundle)
    except (ValueError, KeyError, TypeError):
        return
    raise AssertionError("Unsupported publication witness passed validation")


def test():
    registry, bundle = fixture()
    stats = validate_registry(registry, bundle)
    assert stats == {"historical_publication_witnesses": 1,
                     "empirically_tested_combinations": 0}
    for name, replacement in [
        ("candidate_game_key", "other"),
        ("source_record_id", "unknown"),
        ("source_git_blob_sha", "b" * 40),
        ("source_ordinal", 8),
        ("raw_code", "ABCD-1234+FAKE-0000"),
        ("published_text_segments", ["EFGH-5678", "ABCD-1234"]),
        ("execution_observed", True),
        ("rom_match_verified", True),
        ("safe_to_auto_apply", True),
        ("published_effect_description", ""),
    ]:
        bad = copy.deepcopy(registry)
        bad["records"][0][name] = replacement
        fails(bad, bundle)
    repeated_source = copy.deepcopy(bundle)
    repeated_source["records"].append(copy.deepcopy(repeated_source["records"][0]))
    fails(registry, repeated_source)
    repeated_ordinal = copy.deepcopy(bundle)
    repeated_ordinal["records"][0]["codes"].append(
        copy.deepcopy(repeated_ordinal["records"][0]["codes"][0])
    )
    fails(registry, repeated_ordinal)
    bad = copy.deepcopy(registry)
    bad["records"].append(copy.deepcopy(bad["records"][0]))
    fails(bad, bundle)
    bad = copy.deepcopy(registry)
    bad["records"][0]["publication"]["url"] = "http://example.org/"
    fails(bad, bundle)
    bad = copy.deepcopy(registry)
    bad["records"][0]["publication"]["source_revision"] = ""
    fails(bad, bundle)
    bad_bundle = copy.deepcopy(bundle)
    bad_bundle["records"][0]["codes"][0]["composition"]["relation"] = "revision-alternatives"
    fails(registry, bad_bundle)
    bad_bundle = copy.deepcopy(bundle)
    bad_bundle["records"][0]["provenance"]["git_blob_sha"] = "f" * 40
    fails(registry, bad_bundle)
    print("OK: external SNES publication witnesses are source-exact and not execution claims")


if __name__ == "__main__":
    test()
