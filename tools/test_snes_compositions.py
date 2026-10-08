#!/usr/bin/env python3
"""Adversarial SNES composition fixtures; synthetic strings, no ROMs."""
import copy
import gzip
import json
import tempfile
from pathlib import Path

from validate_snes_compositions import audit


def save(path, obj):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj), encoding="utf-8")


def run_tests():
    with tempfile.TemporaryDirectory(prefix="cheatarium-source-join-fixture-") as work:
        root=Path(work)
        ev={"source_record_id":"fake:record","source_ordinal":2,"source_git_blob_sha":"a"*40,
            "raw_code":"CODE-A+CODE-B+CODE-C+CODE-D","relation":"revision-alternatives",
            "alternatives":[["CODE-A","CODE-B"],["CODE-C","CODE-D"]],
            "evidence":[{"url":"https://example.org/faq","reference":"table 2","source_revision":"v1"}],
            "rom_match_verified":False,"simultaneous_execution_confirmed":False}
        registry={"schema_version":1,"platform":"snes",
            "interpretation":"evidenced-source-code-layout-not-ROM-compatibility",
            "records":[ev]}
        code={"ordinal":2,"role":"code","code":ev["raw_code"],
              "composition":{k:ev[k] for k in ["relation","alternatives","evidence",
                                                "rom_match_verified","simultaneous_execution_confirmed"]}}
        bundle={"schema_version":1,"platform":"snes","records":[
            {"id":"fake:record","provenance":{"git_blob_sha":"a"*40},"codes":[code]}
        ]}
        def attempt(reg, data, expected):
            save(root/"interpretations/v1/snes.json",reg)
            save(root/"generated/v1/interpretations/snes.json",reg)
            with gzip.open(root/"generated/v1/snes.json.gz","wt",encoding="utf-8") as file:
                json.dump(data,file)
            try:
                stats=audit(root)
            except (ValueError, KeyError, TypeError, IndexError):
                if expected:
                    raise AssertionError("Valid source composition rejected")
            else:
                if not expected:
                    raise AssertionError("Corrupted source composition accepted")
                assert stats["reviewed_revision_alternatives"] == 1
        attempt(registry,bundle,True)
        def bad_reg(mut):
            r=copy.deepcopy(registry)
            mut(r["records"][0])
            attempt(r,bundle,False)
        def bad_bundle(mut):
            b=copy.deepcopy(bundle)
            mut(b["records"][0])
            attempt(registry,b,False)
        bad_reg(lambda r:r["alternatives"][1].reverse())
        bad_reg(lambda r:r.update({"rom_match_verified":True}))
        bad_reg(lambda r:r.update({"simultaneous_execution_confirmed":True}))
        bad_reg(lambda r:r["evidence"].clear())
        bad_reg(lambda r:r.update({"source_ordinal":100}))
        r=copy.deepcopy(registry)
        r["records"].append(copy.deepcopy(ev))
        attempt(r,bundle,False)
        bad_bundle(lambda r:r["provenance"].update({"git_blob_sha":"b"*40}))
        bad_bundle(lambda r:r["codes"][0].update({"snes_decode":{"writes":[1,2,3,4]}}))
        bad_bundle(lambda r:r["codes"][0].update({"code":"CODE-CHANGE+CODE-OTHER"}))
        bad_bundle(lambda r:r["codes"][0].update({"composition":None}))
        bad_bundle(lambda r:r["codes"][0].update({"role":"section-heading"}))
        # Generic '+' joins are unresolved, without invented semantics.
        b=copy.deepcopy(bundle)
        b["records"][0]["codes"].append({"ordinal":3,"role":"code","code":"ABC+DEF"})
        attempt(registry,b,False)
        b["records"][0]["codes"][-1]["composition"]={
            "relation":"unresolved","alternatives":[],"evidence":[],
            "rom_match_verified":False,"simultaneous_execution_confirmed":False
        }
        attempt(registry,b,True)
    print("OK: 14 normal and adversarial synthetic SNES composition tests")


if __name__ == "__main__":
    run_tests()
