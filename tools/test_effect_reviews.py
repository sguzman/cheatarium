#!/usr/bin/env python3
"""Positive and adversarial review-registry fixtures, with no actual ROMs."""
import copy
import gzip
import json
from pathlib import Path
import tempfile
from validate_effect_reviews import validate

SHA = "a" * 64
BLOB = "b" * 40

def emit(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj), encoding="utf-8")

def test():
    with tempfile.TemporaryDirectory(prefix="cheatarium-effect-review-fixture-") as tmp:
        root = Path(tmp)
        emit(root / "taxonomy/effects-v1.json", {"categories": [{"id":"lives"}]})
        emit(root / "generated/v1/catalog.json", {"bundles": [{"platform":"snes"}]})
        content = {"schema_version":1, "platform":"snes", "records":[{
            "id":"fixture:record",
            "codes":[{"ordinal":3,"role":"code","description":"Infinite Lives", "code":"DDB4-6F07"},
                     {"ordinal":4,"role":"section-heading","description":"Lives", "code":None}],
            "provenance":{"revision":"upstream-revision", "git_blob_sha":BLOB}
        }]}
        with gzip.open(root / "generated/v1/snes.json.gz", "wt", encoding="utf-8") as f:
            json.dump(content, f)
        claim = {
            "id":"fixture-claim", "platform":"snes", "source_record_id":"fixture:record",
            "source_ordinal":3, "source_revision":"upstream-revision",
            "source_git_blob_sha":BLOB,
            "effect_category":"lives", "assessment":"reported",
            "reviewed_by":"fixture-reviewer", "review_date":"2026-10-08",
            "assessment_note":"Third-party claim, not a test.",
            "evidence":[{"url":"https://example.org/claim", "reference":"entry 1",
                         "source_revision":"fixture-revision"}],
            "test_context":None,
        }
        registry = {"schema_version":1,"format":"cheatarium-effect-reviews-v1",
                    "description":"fixture","claims":[]}
        def run(items, should_pass):
            registry["claims"] = items
            emit(root / "reviews/v1/claims.json", registry)
            try:
                validate(root)
            except (ValueError, KeyError) as err:
                if should_pass:
                    raise AssertionError(f"Valid fixture rejected: {err}") from err
            else:
                if not should_pass:
                    raise AssertionError("Invalid review fixture accepted")
        run([], True)
        run([claim], True)
        observed = copy.deepcopy(claim)
        observed["assessment"] = "observed"
        observed["test_context"] = {"rom_sha256":SHA,"emulator":"Fixture Emulator",
            "emulator_version":"0.1","code_device_or_core":"Fixture core",
            "test_date":"2026-10-08","outcome":"observed",
            "observed_behavior":"Life counter stayed unchanged after loss."}
        run([observed], True)
        unobserved = copy.deepcopy(observed)
        unobserved["assessment"] = "not-reproduced"
        unobserved["test_context"]["outcome"] = "not-reproduced"
        run([unobserved], True)
        for field, bad_value in [
            ("source_record_id","fixture:missing"),
            ("source_ordinal",4),
            ("source_revision","fabricated"),
            ("source_git_blob_sha","bad"),
            ("effect_category","unrecognized"),
            ("assessment","verified"),
            ("review_date","yesterday"),
            ("evidence",[]),
        ]:
            bad=copy.deepcopy(claim)
            bad[field]=bad_value
            run([bad], False)
        bad=copy.deepcopy(observed)
        bad["test_context"]["rom_sha256"]="deadbeef"
        run([bad], False)
        bad=copy.deepcopy(observed)
        bad["test_context"]["outcome"]="not-reproduced"
        run([bad], False)
        bad=copy.deepcopy(observed)
        bad["test_context"]=None
        run([bad], False)
        bad=copy.deepcopy(claim)
        bad["test_context"]=observed["test_context"]
        run([bad], False)
        bad=copy.deepcopy(claim)
        bad["evidence"][0]["url"]="file:///private/rom.sfc"
        run([bad], False)
        run([claim,claim], False)
    print("OK: empty, reported, observed, non-reproduced and 15 adversarial evidence validations")

if __name__ == "__main__":
    test()
