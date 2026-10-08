#!/usr/bin/env python3
"""Test that SNES publication evidence really matches original .cht archive bytes."""
import copy
import hashlib
import tempfile
from pathlib import Path

from audit_snes_publication_archive import (
    archive_path, audit_original_archives, parse_code_fields,
)
from validate_snes_publications import validate_registry, CLAIM_TYPE


SOURCE = "fixture:cht/Game.cht"
RAW = b'cheats = 1\ncheat2_desc = "Level 2"\ncheat2_code = "AAAA+BBBB"\ncheat2_enable = false\n'


def git_blob(data):
    return hashlib.sha1(f"blob {len(data)}".encode() + bytes([0]) + data).hexdigest()


def fixture(sha):
    bundle = {
        "schema_version": 1, "platform": "snes", "records": [{
            "id": SOURCE, "candidate_game_key": "game",
            "provenance": {
                "source_id": "fixture", "upstream_path": "cht/Game.cht",
                "archive_path": "archive/libretro/cht/Game.cht",
                "git_blob_sha": sha,
            },
            "codes": [{"ordinal": 2, "role": "code", "code": "AAAA+BBBB",
                       "composition": {"relation": "unresolved"}}],
        }],
    }
    publications = {
        "schema_version": 1, "platform": "snes", "claim_type": CLAIM_TYPE,
        "evidence_limit": "Historical text, not execution evidence",
        "records": [{
            "candidate_game_key": "game", "source_record_id": SOURCE,
            "source_git_blob_sha": sha, "source_ordinal": 2,
            "raw_code": "AAAA+BBBB", "published_text_segments": ["AAAA", "BBBB"],
            "published_effect_description": "Level 2",
            "publication": {"url": "https://example.org", "reference": "Entry 2",
                            "source_revision": "fixture"},
            "execution_observed": False, "rom_match_verified": False,
            "safe_to_auto_apply": False,
        }],
    }
    return publications, bundle


def rejects(fn):
    try:
        fn()
    except (ValueError, OSError):
        return
    raise AssertionError("Corrupted or unsafe original archive was accepted")


def test():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        file = root / "archive/libretro/cht/Game.cht"
        file.parent.mkdir(parents=True)
        file.write_bytes(RAW)
        publications, bundle = fixture(git_blob(RAW))
        assert validate_registry(publications, bundle)["historical_publication_witnesses"] == 1
        assert audit_original_archives(root, publications, bundle) == {
            "original_archived_files_checked": 1,
            "publication_witness_ordinals_checked": 1,
        }
        assert parse_code_fields(RAW) == {2: "AAAA+BBBB"}
        assert parse_code_fields(RAW + b'cheat3_code="CCCC"\n')[3] == "CCCC"
        rejects(lambda: parse_code_fields(RAW + b'cheat2_code="TAMPER"\n'))

        for suffix in ("../archive/libretro/cht/Game.cht",
                       "/tmp/outsider.cht",
                       "archive/libretro/cht/../Game.cht",
                       "archive/libretro/cht/../../../outside.cht",
                       "archive\\libretro\\cht\\Game.cht"):
            rejects(lambda p=suffix: archive_path(root, p))

        # Mutation of original bytes invalidates the upstream Git blob even if
        # the generated source JSON still looks correct.
        file.write_bytes(RAW.replace(b"AAAA+BBBB", b"AAAA+CCCC"))
        rejects(lambda: audit_original_archives(root, publications, bundle))
        file.write_bytes(RAW)

        altered = copy.deepcopy(publications)
        altered["records"][0]["raw_code"] = "AAAA+CCCC"
        rejects(lambda: audit_original_archives(root, altered, bundle))
        altered = copy.deepcopy(bundle)
        altered["records"][0]["provenance"]["archive_path"] = "../bad.cht"
        rejects(lambda: audit_original_archives(root, publications, altered))
        altered = copy.deepcopy(bundle)
        altered["records"][0]["provenance"]["upstream_path"] = "cht/Another.cht"
        rejects(lambda: audit_original_archives(root, publications, altered))
        altered = copy.deepcopy(bundle)
        altered["records"][0]["provenance"]["git_blob_sha"] = "0" * 40
        rejects(lambda: audit_original_archives(root, publications, altered))
        file.write_bytes(RAW + b'cheat2_code = "AAAA+BBBB"\n')
        altered_sha = git_blob(file.read_bytes())
        amended, updated_bundle = fixture(altered_sha)
        rejects(lambda: audit_original_archives(root, amended, updated_bundle))
        print("OK: archived SNES source bytes, Git SHA, paths and ordinals independently verified")


if __name__ == "__main__":
    test()
