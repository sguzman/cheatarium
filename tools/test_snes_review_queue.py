#!/usr/bin/env python3
"""Synthetic priority-queue tests; never use ROMs."""
from build_snes_review_queue import calculate

def fixture():
    entries=[
        ("game-z","Zeta",0,"AAAA+BBBB","unresolved"),
        ("game-a","Alpha",1,"CCCC+DDDD","unresolved"),
        ("game-a","Alpha",2,"EEEE+FFFF","unresolved"),
        ("game-a","Alpha",3,"GGGG+HHHH","revision-alternatives"),
    ]
    bundle={"schema_version":1,"platform":"snes","records":[]}
    for key,title,ordinal,code,relation in entries:
        record_id=f"fixture:{key}:{ordinal}"
        bundle["records"].append({
            "id":record_id, "candidate_game_key":key,"title_hint":title,
            "region_hint":"USA","format_hint":"game-genie",
            "provenance":{"git_blob_sha":"f"*40},
            "codes":[{"role":"code","code":code,"ordinal":ordinal,
                      "composition":{"relation":relation,"alternatives":[],
                                     "evidence":[]}}]
        })
    docs={"records":[{"source_record_id":"fixture:game-a:3","source_ordinal":3}]}
    report=calculate(bundle,docs)
    assert report["total_unresolved_source_occurrences"]==3
    assert report["candidate_groups_with_unresolved_joins"]==2
    assert [x["candidate_game_key"] for x in report["top_groups"]]==["game-a","game-z"]
    assert report["top_groups"][0]["unresolved_source_occurrences"]==2
    assert report["top_groups"][0]["distinct_source_records"]==2
    assert report["top_groups"][0]["sample_originals"][0]["ordinal"]==1
    assert not report["effect_or_compatibility_verified"]
    publications={
        "schema_version":1,"platform":"snes",
        "claim_type":"externally-published-multi-part-source-code-text",
        "evidence_limit":"Historical text only; no ROM execution evidence",
        "records":[{
            "candidate_game_key":"game-a","source_record_id":"fixture:game-a:1",
            "source_ordinal":1,"source_git_blob_sha":"f"*40,
            "raw_code":"CCCC+DDDD","published_text_segments":["CCCC","DDDD"],
            "published_effect_description":"Example source description",
            "publication":{"url":"https://example.org/manual","reference":"Entry 2",
                           "source_revision":"undated"},
            "execution_observed":False,"rom_match_verified":False,
            "safe_to_auto_apply":False,
        }],
    }
    evidenced=calculate(bundle,docs,publications)
    assert evidenced["total_historical_publication_witnesses"]==1
    assert evidenced["total_without_publication_witness"]==2
    assert evidenced["top_groups"][0]["historical_publication_witnesses"]==1
    assert evidenced["top_groups"][0]["occurrences_without_publication_witness"]==1
    assert evidenced["top_groups"][1]["historical_publication_witnesses"]==0
    publications["records"][0]["source_git_blob_sha"]="b"*40
    try:
        calculate(bundle,docs,publications)
    except ValueError:
        pass
    else:
        raise AssertionError("Queue accepted a tampered external publication witness")

    # An unreviewed code cannot be silently omitted as if it had evidence.
    docs["records"].clear()
    try:
        calculate(bundle,docs)
    except ValueError:
        pass
    else:
        raise AssertionError("Undocumented alternative accepted")
    print("OK: deterministic unresolved-composition queue ranking and evidence boundaries")


if __name__=="__main__":
    fixture()
